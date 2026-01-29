use anyhow::{Context, Result};
use futures::StreamExt;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_util::codec::Framed;

use sentinel_crypto::NodeIdentity;
use sentinel_protocol::{
    messages::{MessageContent, SentinelMessage},
    SentinelCodec,
};
use sentinel_transport::SentinelAcceptor;

struct SentinelNode {
    identity: NodeIdentity,
    acceptor: SentinelAcceptor,
    db: sled::Db,
    mdns: ServiceDaemon,
}

impl SentinelNode {
    async fn new(data_dir: PathBuf) -> Result<Self> {
        let identity = NodeIdentity::load_or_generate(data_dir.join("identity.key"))?;
        let db = sled::open(data_dir.join("storage.db"))?;

        // setup TLS Transport
        let acceptor = SentinelAcceptor::new(
            &data_dir.join("node.crt"),
            &data_dir.join("node.key"),
            Duration::from_secs(10),
        )?;

        let mdns = ServiceDaemon::new().context("Failed to create mDNS daemon")?;

        Ok(Self {
            identity,
            acceptor,
            db,
            mdns,
        })
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
                if let Ok(bytes) = item {
                    if let Ok(msg) = SentinelMessage::from_bytes(&bytes) {
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

    fn start_discovery(&self, port: u16) -> Result<()> {
        let mdns = self.mdns.clone();
        let node_id = self.identity.node_id();
        let service_type = "_sentinel._tcp.local.";
        
        // register this node so others find .......
        let instance_name = format!("{}.sentinel", node_id);
        let my_service = ServiceInfo::new(
            service_type,
            &instance_name,
            "sentinel-node.local.",
            "",
            port,
            None,
        )?;
        mdns.register(my_service)?;

        // Browse for other nodes
        let receiver = mdns.browse(service_type)?;
        tokio::spawn(async move {
            while let Ok(event) = receiver.recv_async().await {
                if let ServiceEvent::ServiceResolved(info) = event {
                    println!("mDNS: Discovered Peer -> {}", info.get_fullname());
                    // will pass this info to a 'connect' function
                }
            }
        });

        Ok(())
    }

    async fn run(&self, addr: &str, port: u16) -> Result<()> {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .ok(); 

        self.print_history()?;
        self.start_discovery(port)?;

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
                            if let MessageContent::Chat(text) = &msg.conte
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
    let node = SentinelNode::new(PathBuf::from("./.sentinel")).await?;
    node.run("0.0.0.0:8443", 8443).await
}