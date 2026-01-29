use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use dashmap::DashMap;
use tokio::sync::mpsc;
use uuid::Uuid;
use std::sync::Arc;
use futures::{StreamExt, SinkExt};
use tokio_util::codec::Framed;

use sentinel_crypto::NodeIdentity;
use sentinel_protocol::{
    SentinelCodec, 
    frame::Frame,
    messages::{SentinelMessage, MessageContent}
};
use sentinel_transport::{SentinelAcceptor, SentinelConnector};
use mdns_sd::ServiceDaemon;

pub struct SentinelNode {
    pub identity: NodeIdentity,
    pub acceptor: SentinelAcceptor,
    pub db: sled::Db,
    pub mdns: ServiceDaemon,
    pub peers: DashMap<String, mpsc::UnboundedSender<SentinelMessage>>,
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

        Ok(Self { 
            identity, 
            acceptor, 
            db, 
            mdns,
            peers: DashMap::new(),
        })
    }

    pub async fn dial_peer(self: Arc<Self>, addr: String) -> Result<()> {
        let connector = SentinelConnector::new(&PathBuf::from("./.sentinel/node.crt"))?;
        let stream = tokio::net::TcpStream::connect(&addr).await?;
        let tls = connector.connect("sentinel-node.local", stream).await?;
        
        let (mut sink, mut stream) = Framed::new(tls, SentinelCodec::new()).split();

        // 1. Send Handshake
        let handshake = SentinelMessage {
            id: Uuid::new_v4(),
            sender: self.identity.node_id(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            content: MessageContent::Chat("🤝 Handshake Connected".to_string()),
        };
        
        let frame = Frame::new(1, 0, bytes::Bytes::from(handshake.to_bytes()))?;
        sink.send(frame).await?;

        // 2. Setup Peer Channel
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.peers.insert(addr.clone(), tx);

        // 3. Outgoing Message Task
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Ok(f) = Frame::new(1, 0, bytes::Bytes::from(msg.to_bytes())) {
                    let _ = sink.send(f).await;
                }
            }
        });

        // 4. Incoming Message Task
        let node_inner = Arc::clone(&self);
        tokio::spawn(async move {
            while let Some(Ok(frame)) = stream.next().await {
                if let Ok(msg) = SentinelMessage::from_bytes(frame.payload()) {
                    // Use {:?} here for MessageContent formatting
                    println!("[{}] {:?}", msg.sender, msg.content);
                    let _ = node_inner.persist_message(&msg);
                }
            }
            node_inner.peers.remove(&addr);
        });

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
        println!("--- RECENT HISTORY ---");
        if let Ok(tree) = self.db.open_tree("messages") {
            for item in tree.iter().values().rev().take(10) {
                if let Ok(bytes) = item {
                    if let Ok(msg) = SentinelMessage::from_bytes(&bytes) {
                        println!("[{}] {:?}", msg.sender, msg.content);
                    }
                }
            }
        }
        println!("----------------------");
        Ok(())
    }
}