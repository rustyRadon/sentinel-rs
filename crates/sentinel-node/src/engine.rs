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

/// Metadata for a connected peer
pub struct PeerState {
    pub tx: mpsc::UnboundedSender<SentinelMessage>,
    pub node_id: String,
    pub node_name: String,
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
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)?;
        }

        let identity = NodeIdentity::load_or_generate(data_dir.join("identity.key"))?;
        
        let db = sled::open(data_dir.join("storage.db"))?;

        let cert_path = if data_dir.join("node.crt").exists() {
            data_dir.join("node.crt")
        } else {
            PathBuf::from("certs/server.crt") 
        };

        let key_path = if data_dir.join("node.key").exists() {
            data_dir.join("node.key")
        } else {
            PathBuf::from("certs/server.key")
        };

        if !cert_path.exists() || !key_path.exists() {
            return Err(anyhow::anyhow!(
                "TLS certificates not found in {:?} or project root. Please ensure node.crt and node.key exist.",
                data_dir
            ));
        }

        let acceptor = SentinelAcceptor::new(
            &cert_path,
            &key_path,
            Duration::from_secs(10),
        )?;

        let mdns = ServiceDaemon::new().context("Failed to start mDNS")?;
        let seen_messages = Mutex::new(LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()));

        Ok(Self { 
            identity, 
            acceptor, 
            db, 
            mdns, 
            peers: DashMap::new(), 
            seen_messages 
        })
    }

    /// handles incoming messages. 
    /// returns BoxFuture to break the async recursion cycle with dial_peer.
    pub fn handle_incoming_message(self: Arc<Self>, msg: SentinelMessage, addr: String) -> BoxFuture<'static, Result<()>> {
        let node = self.clone();
        async move {
            {
                let mut seen = node.seen_messages.lock().await;
                if seen.contains(&msg.id) { return Ok(()); }
                seen.put(msg.id, ());
            }

            match msg.content {
                MessageContent::Chat(ref text) => {
                    println!("[{}] (Chat): {}", msg.sender, text);
                    let _ = node.persist_message(&msg);

                    for entry in node.peers.iter() {
                        let peer_addr = entry.key();
                        if peer_addr != &addr {
                            let _ = entry.value().tx.send(msg.clone());
                        }
                    }
                }
                MessageContent::PeerDiscovery(ref new_peers) => {
                    for peer in new_peers {
                        let addr_str = peer.address.to_string();

                        if peer.node_id == node.identity.node_id() {
                            continue;
                        }

                        if !node.peers.contains_key(&addr_str) {
                            println!("Auto-connecting to discovered peer: {} ({})", peer.node_name, addr_str);
                            
                            let node_clone = Arc::clone(&node);
                            tokio::spawn(async move {
                                if let Err(e) = node_clone.dial_peer(addr_str).await {
                                    eprintln!("Failed to autoconnect: {}", e);
                                }
                            });
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
        let connector = SentinelConnector::new(&PathBuf::from("./node.crt"))?;
        let stream = tokio::net::TcpStream::connect(&addr).await?;
        let tls = connector.connect("sentinel-node.local", stream).await?;
        let (mut sink, mut stream) = Framed::new(tls, SentinelCodec::new()).split();

        let handshake = SentinelMessage::new(self.identity.node_id(), MessageContent::Chat("v2-dial".into()));
        sink.send(Frame::new(1, 0, handshake.to_bytes().into())?).await?;

        let (tx, mut rx) = mpsc::unbounded_channel();
        
        self.peers.insert(addr.clone(), PeerState {
            tx,
            node_id: "pending".into(),
            node_name: "new-peer".into(),
        });

        let addr_out = addr.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Ok(f) = Frame::new(1, 0, msg.to_bytes().into()) {
                    if let Err(e) = sink.send(f).await {
                        eprintln!("Write error to peer {}: {}", addr_out, e);
                        break; 
                    }
                }
            }
        });

        let node_inner = Arc::clone(&self);
        let addr_in = addr.clone();
        tokio::spawn(async move {
            while let Some(Ok(frame)) = stream.next().await {
                if let Ok(msg) = SentinelMessage::from_bytes(frame.payload()) {
                    if let Some(mut peer) = node_inner.peers.get_mut(&addr_in) {
                        if peer.node_id == "pending" {
                            peer.node_id = msg.sender.clone();
                        }
                    }
                    let _ = node_inner.clone().handle_incoming_message(msg, addr_in.clone()).await;
                }
            }
            node_inner.peers.remove(&addr_in);
            println!("Connection closed: {}", addr_in);
        });
        Ok(())
    }

    pub async fn start_gossip_service(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let peer_list: Vec<PeerInfo> = self.peers.iter().filter_map(|entry| {
                let state = entry.value();
                entry.key().parse().ok().map(|addr| PeerInfo {
                    node_id: state.node_id.clone(),
                    address: addr,
                    node_name: state.node_name.clone(),
                    last_seen: 0,
                })
            }).collect();

            if !peer_list.is_empty() {
                let msg = MessageContent::PeerDiscovery(peer_list);
                for entry in self.peers.iter() {
                    let _ = entry.value().tx.send(SentinelMessage::new(self.identity.node_id(), msg.clone()));
                }
            }
        }
    }

    pub async fn send_to_peer(&self, addr: &str, content: MessageContent) -> Result<()> {
        if let Some(peer) = self.peers.get(addr) {
            peer.tx.send(SentinelMessage::new(self.identity.node_id(), content))?;
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