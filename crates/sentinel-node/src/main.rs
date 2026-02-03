mod engine;
mod discovery;
mod handlers;

use anyhow::Result;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use futures::{StreamExt, SinkExt};
use sentinel_protocol::{
    SentinelCodec, 
    frame::Frame, 
    messages::SentinelMessage
};
use crate::engine::SentinelNode;
use clap::Parser; // 1. Add this import

// 2. Define the CLI arguments
#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "./.sentinel")]
    data_dir: PathBuf,

    #[arg(short, long, default_value_t = 8443)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::aws_lc_rs::default_provider().install_default().ok();

    // 3. Parse the arguments from the terminal
    let args = Args::parse();

    // 4. Use the parsed data_dir and port
    let node = Arc::new(SentinelNode::new(args.data_dir).await?);
    node.print_history()?;
    
    // Use the custom port for mDNS discovery
    node.start_discovery(args.port)?;

    let gossip_node = Arc::clone(&node);
    tokio::spawn(async move { gossip_node.start_gossip_service().await });

    let stdin_node = Arc::clone(&node);
    tokio::spawn(async move { let _ = handlers::spawn_stdin_handler(stdin_node).await; });

    // 5. Bind to the custom port
    let addr = format!("0.0.0.0:{}", args.port);
    let listener = TcpListener::bind(&addr).await?;
    println!("RUNNING ON {}", addr);

    loop {
        let (stream, addr) = listener.accept().await?;
        let acceptor = node.acceptor.clone();
        let node_inner = Arc::clone(&node);
        let addr_str = addr.to_string();

        tokio::spawn(async move {
            if let Ok(tls) = acceptor.accept(stream).await {
                let (mut sink, mut stream) = Framed::new(tls, SentinelCodec::new()).split();
                let (tx, mut rx) = mpsc::unbounded_channel::<SentinelMessage>();
                let mut peer_id = String::from("unknown");

                tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        if let Ok(f) = Frame::new(1, 0, msg.to_bytes().into()) {
                            if sink.send(f).await.is_err() { break; }
                        }
                    }
                });

                while let Some(Ok(frame)) = stream.next().await {
                    if let Ok(msg) = SentinelMessage::from_bytes(frame.payload()) {
                        if peer_id == "unknown" {
                            peer_id = msg.sender.clone();
                            node_inner.peers.insert(addr_str.clone(), engine::PeerState {
                                tx: tx.clone(),
                                node_id: peer_id.clone(),
                                node_name: "Inbound-Peer".into(),
                            });
                        }
                        let _ = node_inner.clone().handle_incoming_message(msg, addr_str.clone()).await;
                    }
                }
                node_inner.peers.remove(&addr_str);
            }
        });
    }
}