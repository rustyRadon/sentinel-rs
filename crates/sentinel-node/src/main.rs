use rustls; 
use anyhow::{ Result};
use std::path::PathBuf;
use std::time::{Duration};
use tokio::net::TcpListener;
use tokio_util::codec::Framed;
use futures::StreamExt;

use sentinel_crypto::NodeIdentity;
use sentinel_transport::SentinelAcceptor;
use sentinel_protocol::{
    messages::{SentinelMessage, MessageContent},
    SentinelCodec,
};

struct SentinelNode {
    identity: NodeIdentity,
    acceptor: SentinelAcceptor,
    db: sled::Db,
}

impl SentinelNode {
    async fn new(data_dir: PathBuf) -> Result<Self> {
        let identity = NodeIdentity::load_or_generate(data_dir.join("identity.key"))?;
        let db = sled::open(data_dir.join("storage.db"))?;
        
        let acceptor = SentinelAcceptor::new(
            &data_dir.join("node.crt"),
            &data_dir.join("node.key"),
            Duration::from_secs(10),
        )?;

        Ok(Self { identity, acceptor, db })
    }

    fn persist_message(&self, msg: &SentinelMessage) -> Result<()> {
        let tree = self.db.open_tree("messages")?;
        let key = format!("{}:{}", msg.timestamp, msg.sender);
        tree.insert(key, msg.to_bytes())?;
        tree.flush()?;
        Ok(())
    }

    fn print_history(&self) -> Result<()> {
        println!("--- RECENT HISTORY ---");
        if let Ok(tree) = self.db.open_tree("messages") {
            for item in tree.iter().values().rev().take(10) {
                if let Ok(bytes_ivec) = item {
                    if let Ok(msg) = SentinelMessage::from_bytes(&bytes_ivec) {
                        if let MessageContent::Chat(text) = msg.content {
                            println!("[{}] {}", msg.sender, text);
                        }
                    }
                }
            }
        }
        println!("----------------------");
        Ok(())
    }

    async fn run(&self, addr: &str) -> Result<()> {
        self.print_history()?;

        let listener = TcpListener::bind(addr).await?;
        println!("SENTINEL ACTIVE | ID: {} | ADDR: {}", self.identity.node_id(), addr);

        loop {
            let (stream, _peer_addr) = listener.accept().await?;
            let acceptor = self.acceptor.clone();
            let db = self.db.clone();

            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(stream).await {
                    let mut framed = Framed::new(tls, SentinelCodec::new());
                    while let Some(Ok(frame)) = framed.next().await {
                        if let Ok(msg) = SentinelMessage::from_bytes(frame.payload()) {
                            if let MessageContent::Chat(text) = &msg.content {
                                println!("[{}] {}", msg.sender, text);
                                
                                let tree = db.open_tree("messages").unwrap();
                                tree.insert(format!("{}:{}", msg.timestamp, msg.sender), msg.to_bytes()).unwrap();
                            }
                        }
                    }
                }
            });
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    //  use aws-lc-rs as the default crypto provider
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install crypto provider");

    let data_dir = PathBuf::from("./.sentinel");
    let node = SentinelNode::new(data_dir).await?;
    node.run("0.0.0.0:8443").await?;
    Ok(())
}