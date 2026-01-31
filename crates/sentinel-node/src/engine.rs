use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use dashmap::DashMap;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use std::sync::Arc;
use futures::{StreamExt, SinkExt};
use tokio_util::codec::Framed;
use lru::LruCache; // Add `lru = "0.12"` to Cargo.toml

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

        Ok(Self { 
            identity, 
            acceptor, 
            db, 
            mdns,
            peers: DashMap::new(),
            seen_messages,
        })
    }

    async fn handle_incoming_message(self: Arc<Self>, msg: SentinelMessage, addr: String) -> Result<()> {
        {
            let mut seen = self.seen_messages.lock().await;
            if seen.contains(&msg.id) {
                return Ok(()); 
            }
            seen.put(msg.id, ());
        }

        match msg.content {
            MessageContent::Chat(text) => {
                println!("[{}] (Chat): {}", msg.sender, text);
                self.persist_message(&msg)?;
            }
            MessageContent::PeerDiscovery(new_peers) => {
                println!("[{}] Shared {} peers with us", msg.sender, new_peers.len());
                for peer in new_peers {
                    self.process_discovered_peer(peer).await;
                }
            }
            MessageContent::Ping => {
                self.send_to_peer(&addr, MessageContent::Pong).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn process_discovered_peer(&self, peer: PeerInfo) {
        if peer.node_id == self.identity.node_id() { return; }
        let addr_str = peer.address.to_string();
        
        if !self.peers.contains_key(&addr_str) {
            println!("✨ Discovered new peer via gossip: {} at {}", peer.node_name, addr_str);
        }
    }

    pub async fn dial_peer(self: Arc<Self>, addr: String) -> Result<()> {
        let connector = SentinelConnector::new(&PathBuf::from("./node.crt"))?;
        let stream = tokio::net::TcpStream::connect(&addr).await?;
        let tls = connector.connect("sentinel-node.local", stream).await?;
        
        let (mut sink, mut stream) = Framed::new(tls, SentinelCodec::new()).split();

        let handshake = SentinelMessage::new(
            self.identity.node_id(),
            MessageContent::Chat(" Connected to Mesh".to_string())
        );
        
        let frame = Frame::new(1, 0, bytes::Bytes::from(handshake.to_bytes()))?;
        sink.send(frame).await?;

        let (tx, mut rx) = mpsc::unbounded_channel();
        self.peers.insert(addr.clone(), tx);

        // Task: Outgoing Data
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Ok(f) = Frame::new(1, 0, bytes::Bytes::from(msg.to_bytes())) {
                    if let Err(e) = sink.send(f).await {
                        eprintln!("Failed to send to {}: {}", addr, e);
                        break;
                    }
                }
            }
        });

        // Task: Incoming Data
        let node_inner = Arc::clone(&self);
        let addr_inner = addr.clone();
        tokio::spawn(async move {
            while let Some(Ok(frame)) = stream.next().await {
                if let Ok(msg) = SentinelMessage::from_bytes(frame.payload()) {
                    let _ = node_inner.clone().handle_incoming_message(msg, addr_inner.clone()).await;
                }
            }
            node_inner.peers.remove(&addr_inner);
        });

        Ok(())
    }

    pub async fn send_to_peer(&self, addr: &str, content: MessageContent) -> Result<()> {
        if let Some(tx) = self.peers.get(addr) {
            let msg = SentinelMessage::new(self.identity.node_id(), content);
            tx.send(msg)?;
        }
        Ok(())
    }

    pub fn persist_message(&self, msg: &SentinelMessage) -> Result<()> {
        let tree = self.db.open_tree("messages")?;
        let key = format!("{}:{}", msg.timestamp, msg.sender);
        tree.insert(key, msg.to_bytes())?;
        tree.flush()?;
        Ok(())
    }
    
    pub fn print_history(&self) -> Result<()> {
        let tree = self.db.open_tree("messages")?;
        for item in tree.iter() {
            let (key, value) = item?;
            if let Ok(msg) = SentinelMessage::from_bytes(&value) {
                println!("[{}] {}: {:?}", msg.timestamp, msg.sender, msg.content);
            }
        }
        Ok(())
    }
}