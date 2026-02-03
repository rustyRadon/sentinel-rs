use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;
use dashmap::DashMap;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use std::sync::Arc;
use futures::{StreamExt, SinkExt, future::{BoxFuture, FutureExt}}; 
use tokio_util::codec::Framed;
use lru::LruCache;

use sentinel_crypto::NodeIdentity;
use sentinel_protocol::{
    SentinelCodec, 
    frame::Frame,
    messages::{SentinelMessage, MessageContent, PeerInfo}
};
use sentinel_transport::{SentinelAcceptor, SentinelConnector};
use mdns_sd::ServiceDaemon;

pub struct PeerState {
    pub tx: mpsc::UnboundedSender<SentinelMessage>,
    pub node_id: String,
    pub node_name: String,
    pub public_key: Option<Vec<u8>>,
}

pub struct SentinelNode {
    pub identity: NodeIdentity,
    pub acceptor: SentinelAcceptor,
    pub db: sled::Db,
    pub mdns: ServiceDaemon,
    pub peers: DashMap<String, PeerState>,
    pub seen_messages: Mutex<LruCache<Uuid, ()>>,
}

impl SentinelNode {
    pub async fn new(data_dir: PathBuf) -> Result<Self> {
        if !data_dir.exists() { std::fs::create_dir_all(&data_dir)?; }
        let identity = NodeIdentity::load_or_generate(data_dir.join("identity.key"))?;
        let db = sled::open(data_dir.join("storage.db"))?;
        let cert_path = if data_dir.join("node.crt").exists() { data_dir.join("node.crt") } else { PathBuf::from("certs/server.crt") };
        let key_path = if data_dir.join("node.key").exists() { data_dir.join("node.key") } else { PathBuf::from("certs/server.key") };
        let acceptor = SentinelAcceptor::new(&cert_path, &key_path, Duration::from_secs(10))?;
        let mdns = ServiceDaemon::new().context("mDNS failed")?;
        let seen_messages = Mutex::new(LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()));

        Ok(Self { identity, acceptor, db, mdns, peers: DashMap::new(), seen_messages })
    }

    pub fn sign_and_send(&self, tx: &mpsc::UnboundedSender<SentinelMessage>, mut msg: SentinelMessage) {
        msg.public_key = self.identity.public_key_bytes();
        msg.signature = self.identity.sign(&msg.sig_hash());
        let _ = tx.send(msg);
    }

    pub fn handle_incoming_message(self: Arc<Self>, msg: SentinelMessage, addr: String) -> BoxFuture<'static, Result<()>> {
        let node = self.clone();
        async move {
            {
                let mut seen = node.seen_messages.lock().await;
                if seen.contains(&msg.id) { return Ok(()); }
                seen.put(msg.id, ());
            }

            // verify signature
            if !msg.signature.is_empty() && !msg.public_key.is_empty() {
                if !NodeIdentity::verify(&msg.sig_hash(), &msg.signature, &msg.public_key) {
                    eprintln!(" Invalid signature from {}", msg.sender);
                    return Ok(());
                }
            }

            // Clone content to own the data for the match block
            let content = msg.content.clone();

            match content {
                MessageContent::Handshake { public_key, node_name } => {
                    println!(" Peer Verified: {} as {}", node_name, msg.sender);
                    if let Some(mut peer) = node.peers.get_mut(&addr) {
                        peer.node_id = msg.sender.clone();
                        peer.node_name = node_name;
                        peer.public_key = Some(public_key);
                    }
                }
                MessageContent::Chat(text) => {
                    println!("[{}] (Chat): {}", msg.sender, text);
                    let _ = node.persist_message(&msg);
                    for entry in node.peers.iter() {
                        if entry.key() != &addr {
                            let _ = entry.value().tx.send(msg.clone());
                        }
                    }
                }
                MessageContent::PeerDiscovery(new_peers) => {
                    for peer in new_peers {
                        if peer.node_id != node.identity.node_id() && !node.peers.contains_key(&peer.address.to_string()) {
                            let node_clone = Arc::clone(&node);
                            tokio::spawn(async move { let _ = node_clone.dial_peer(peer.address.to_string()).await; });
                        }
                    }
                }
                MessageContent::Ping => {
                    let _ = node.send_to_peer(&addr, MessageContent::Pong).await;
                }
                _ => {}
            }
            Ok(())
        }.boxed() 
    }

    pub async fn dial_peer(self: Arc<Self>, addr: String) -> Result<()> {
        let connector = SentinelConnector::new(&PathBuf::from("certs/server.crt"))?;
        let stream = tokio::net::TcpStream::connect(&addr).await?;
        let tls = connector.connect("sentinel-node.local", stream).await?;
        let (mut sink, mut stream) = Framed::new(tls, SentinelCodec::new()).split();

        let (tx, mut rx) = mpsc::unbounded_channel();
        self.peers.insert(addr.clone(), PeerState { tx: tx.clone(), node_id: "pending".into(), node_name: "new-peer".into(), public_key: None });

        let hs = SentinelMessage::new(self.identity.node_id(), MessageContent::Handshake {
            public_key: self.identity.public_key_bytes(),
            node_name: "Sentinel-Node".into(),
        });
        self.sign_and_send(&tx, hs);

        let addr_io = addr.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Ok(f) = Frame::new(1, 0, msg.to_bytes().into()) {
                    if sink.send(f).await.is_err() { break; }
                }
            }
        });

        let node_inner = Arc::clone(&self);
        tokio::spawn(async move {
            while let Some(Ok(frame)) = stream.next().await {
                if let Ok(msg) = SentinelMessage::from_bytes(frame.payload()) {
                    let _ = node_inner.clone().handle_incoming_message(msg, addr_io.clone()).await;
                }
            }
            node_inner.peers.remove(&addr_io);
        });
        Ok(())
    }

    pub async fn start_gossip_service(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let peer_list: Vec<PeerInfo> = self.peers.iter().filter_map(|e| {
                e.key().parse().ok().map(|addr| PeerInfo {
                    node_id: e.value().node_id.clone(),
                    address: addr,
                    node_name: e.value().node_name.clone(),
                    last_seen: 0,
                })
            }).collect();

            if !peer_list.is_empty() {
                let msg = SentinelMessage::new(self.identity.node_id(), MessageContent::PeerDiscovery(peer_list));
                for entry in self.peers.iter() { self.sign_and_send(&entry.value().tx, msg.clone()); }
            }
        }
    }

    pub async fn send_to_peer(&self, addr: &str, content: MessageContent) -> Result<()> {
        if let Some(peer) = self.peers.get(addr) {
            self.sign_and_send(&peer.tx, SentinelMessage::new(self.identity.node_id(), content));
        }
        Ok(())
    }

    pub fn persist_message(&self, msg: &SentinelMessage) -> Result<()> {
        let tree = self.db.open_tree("messages")?;
        tree.insert(format!("{}:{}", msg.timestamp, msg.sender), msg.to_bytes())?;
        Ok(())
    }

    pub fn print_history(&self) -> Result<()> {
        if let Ok(tree) = self.db.open_tree("messages") {
            for item in tree.iter().values().rev().take(10).flatten() {
                if let Ok(msg) = SentinelMessage::from_bytes(&item) {
                    println!("[{}] {:?}", msg.sender, msg.content);
                }
            }
        }
        Ok(())
    }
}