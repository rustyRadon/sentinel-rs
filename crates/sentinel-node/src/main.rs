use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_util::codec::Framed;

use sentinel_crypto::NodeIdentity;
use sentinel_transport::SentinelAcceptor;
use sentinel_protocol::SentinelCodec;

struct SentinelNode {
    identity: NodeIdentity,
    acceptor: SentinelAcceptor,
}

impl SentinelNode {
    async fn new(data_dir: PathBuf) -> Result<Self> {
        let key_path = data_dir.join("identity.key");
        let identity = NodeIdentity::load_or_generate(key_path)?;

        // for Phase 1, these must exist in data_dir
        // can generate them with openssl or automate it later
        let cert_path = data_dir.join("node.crt");
        let tls_key_path = data_dir.join("node.key");

        let acceptor = SentinelAcceptor::new(
            &cert_path,
            &tls_key_path,
            Duration::from_secs(10),
        )?;

        Ok(Self { identity, acceptor })
    }

    async fn run(&self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("SENTINEL P2P NODE ACTIVE");
        println!("NODE ID: {}", self.identity.node_id());
        println!("LISTENING ON: {}", addr);

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let acceptor = self.acceptor.clone();

            tokio::spawn(async move {
                println!("New connection from: {}", peer_addr);
                
                // 1. perform TLS Handshake using sentinel-transport
                match acceptor.accept(stream).await {
                    Ok(tls_transport) => {
                        let mut framed = Framed::new(tls_transport, SentinelCodec::new());
                        
                        while let Some(result) = framed.next().await {
                            match result {
                                Ok(frame) => {
                                    if let Ok(msg) = SentinelMessage::from_bytes(frame.payload()) {
                                        match msg.content {
                                            MessageContent::Chat(text) => {
                                                println!("[{}] Chat: {}", msg.sender, text);
                                                // where DDIA 5.4 comes in. gotta read up man
                                                // forward this message to OTHER peers (Replication)
                                            },
                                            MessageContent::Ping => {
                                                // Respond with Pong...
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Err(e) => eprintln!("Protocol error: {:?}", e),
                            }
                        }
                    }
                    Err(e) => eprintln!("Transport error: {:?}", e),
                }
            });
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let data_dir = PathBuf::from("./.sentinel");
    let node = SentinelNode::new(data_dir).await?;
    
    node.run("0.0.0.0:8443").await?;
    
    Ok(())
}