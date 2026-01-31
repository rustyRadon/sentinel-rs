use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;
use dashmap::DashMap;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use std::sync::Arc;
use futures::{StreamExt, SinkExt};
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

pub struct SentinelNode {
    pub identity: NodeIdentity,
    pub acceptor: SentinelAcceptor,
    pub db: sled::Db,
    pub mdns: ServiceDaemon,
    pub peers: DashMap<String, mpsc::UnboundedSender<SentinelMessage>>,
    pub seen_messages: Mutex<LruCache<Uuid, ()>>,
}

impl SentinelNode {
    pub async fn new(data_dir: PathBuf) -> Result<Self> {
        let identity = NodeIdentity::load_or_generate(data_dir.join("identity.key"))?;
        let db = sled::open(data_dir.join("storage.db"))?;
        let acceptor = SentinelAcceptor::new(
            &data_dir.join("node.crt"),
            &data_dir.join("node.key"),
            Duration::from_secs(10),
        )?;
        let mdns = ServiceDaemon::new().context("Failed to start mDNS")?;
        let seen_messages = Mutex::new(LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()));

        Ok(Self { identity, acceptor, db, mdns, peers: DashMap::new(), seen_messages })
    }

    pub async fn handle_incoming_message(self: Arc<Self>, msg: SentinelMessage, addr: String) -> Result<()> {
        {
            let mut seen = self.seen_messages.lock().await;
            if seen.contains(&msg.id) { return Ok(()); }
            seen.put(msg.id, ());
        }

        match msg.content {
            MessageContent::Chat(ref text) => {
                println!("[{}] (Chat): {}", msg.sender, text);
                self.persist_message(&msg)?;
            }
            MessageContent::PeerDiscovery(ref new_peers) => {
                for peer in new_peers {
                    if peer.node_id != self.identity.node_id() {
                        println!("Gossip discovery: {} at {}", peer.node_name, peer.address);
                    }
                }
            }
            MessageContent::Ping => {
                let _ = self.send_to_peer(&addr, MessageContent::Pong).await;
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn dial_peer(self: Arc<Self>, addr: String) -> Result<()> {
        let connector = SentinelConnector::new(&PathBuf::from("./node.crt"))?;
        let stream = tokio::net::TcpStream::connect(&addr).await?;
        let tls = connector.connect("sentinel-node.local", stream).await?;
        let (mut sink, mut stream) = Framed::new(tls, SentinelCodec::new()).split();

        let handshake = SentinelMessage::new(self.identity.node_id(), MessageContent::Chat("v2-dial".into()));
        sink.send(Frame::new(1, 0, handshake.to_bytes().into())?).await?;

        let (tx, mut rx) = mpsc::unbounded_channel();
        self.peers.insert(addr.clone(), tx);

        let addr_out = addr.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Ok(f) = Frame::new(1, 0, msg.to_bytes().into()) {
                    if sink.send(f).await.is_err() { break; }
                }
            }
        });

        let node_inner = Arc::clone(&self);
        let addr_in = addr.clone();
        tokio::spawn(async move {
            while let Some(Ok(frame)) = stream.next().await {
                if let Ok(msg) = SentinelMessage::from_bytes(frame.payload()) {
                    let _ = node_inner.clone().handle_incoming_message(msg, addr_in.clone()).await;
                }
            }
            node_inner.peers.remove(&addr_in);
        });
        Ok(())
    }

    pub async fn start_gossip_service(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let peer_list: Vec<PeerInfo> = self.peers.iter().filter_map(|entry| {
                entry.key().parse().ok().map(|addr| PeerInfo {
                    node_id: "unknown".into(),
                    address: addr,
                    node_name: "mesh-node".into(),
                    last_seen: 0,
                })
            }).collect();

            if !peer_list.is_empty() {
                let msg = MessageContent::PeerDiscovery(peer_list);
                for entry in self.peers.iter() {
                    let _ = self.send_to_peer(entry.key(), msg.clone()).await;
                }
            }
        }
    }

    pub async fn send_to_peer(&self, addr: &str, content: MessageContent) -> Result<()> {
        if let Some(tx) = self.peers.get(addr) {
            tx.send(SentinelMessage::new(self.identity.node_id(), content))?;
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