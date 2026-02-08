use anyhow::{Context, Result};
use clap::Parser;
use futures::{SinkExt, StreamExt};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;

mod discovery;
mod engine;
mod handlers;

use crate::engine::{SentinelNode, PeerState};
use sentinel_protocol::{
    SentinelCodec, 
    messages::{SentinelMessage, MessageContent}
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./.sentinel")]
    data_dir: PathBuf,

    #[arg(short, long, default_value_t = 8443)]
    port: u16,

    #[arg(short, long, default_value = "127.0.0.1:8888")]
    signaler: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let args = Args::parse();
    let node = Arc::new(SentinelNode::new(args.data_dir).await?);
    let addr = format!("0.0.0.0:{}", args.port);
    let listener = TcpListener::bind(&addr).await
        .context(format!("Failed to bind to {}", addr))?;

    println!("RUNNING ON {}", addr);
    println!("NODE ID: {}", node.identity.node_id());

    discovery::start_discovery(Arc::clone(&node), args.port).await?;

    // Phase 3: Start Signaler Client
    let signaler_node = Arc::clone(&node);
    let signaler_addr = args.signaler.clone();
    tokio::spawn(async move {
        signaler_node.start_signaler_client(signaler_addr).await;
    });

    let gossip_node = Arc::clone(&node);
    tokio::spawn(async move {
        gossip_node.start_gossip_service().await;
    });

    let server_node = Arc::clone(&node);
    tokio::spawn(async move {
        loop {
            if let Ok((stream, remote_addr)) = listener.accept().await {
                let acceptor = server_node.acceptor.clone();
                let node_inner = Arc::clone(&server_node);
                let addr_str = remote_addr.to_string();

                tokio::spawn(async move {
                    if let Ok(tls) = acceptor.accept(stream).await {
                        let (mut sink, mut stream) = Framed::new(tls, SentinelCodec::new()).split();
                        let (tx, mut rx) = mpsc::unbounded_channel::<SentinelMessage>();

                        let hs = SentinelMessage::new(
                            node_inner.identity.node_id(), 
                            MessageContent::Handshake {
                                public_key: node_inner.identity.public_key_bytes(),
                                node_name: "Sentinel-Node".into(),
                            }
                        );
                        node_inner.sign_and_send(&tx, hs);

                        node_inner.peers.insert(addr_str.clone(), PeerState {
                            tx, node_id: "pending".into(), node_name: "Inbound-Peer".into(), public_key: None,
                        });

                        tokio::spawn(async move {
                            while let Some(msg) = rx.recv().await {
                                if sink.send(msg).await.is_err() { break; }
                            }
                        });

                        while let Some(Ok(msg)) = stream.next().await {
                            let _ = node_inner.clone().handle_incoming_message(msg, addr_str.clone()).await;
                        }
                        node_inner.peers.remove(&addr_str);
                    }
                });
            }
        }
    });

    println!("READY TO CHAT. Type and hit Enter.");
    handlers::handle_stdin(Arc::clone(&node)).await?;
    Ok(())
}