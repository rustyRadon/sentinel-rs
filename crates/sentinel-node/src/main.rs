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

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::aws_lc_rs::default_provider().install_default().ok();

    let node = Arc::new(SentinelNode::new(PathBuf::from("./.sentinel")).await?);
    node.print_history()?;
    node.start_discovery(8443)?;

    let gossip_node = Arc::clone(&node);
    tokio::spawn(async move { gossip_node.start_gossip_service().await });

    let stdin_node = Arc::clone(&node);
    tokio::spawn(async move { let _ = handlers::spawn_stdin_handler(stdin_node).await; });

    let listener = TcpListener::bind("0.0.0.0:8443").await?;
    println!("RUNNING ON 8443");

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
                            node_inner.peers.insert(addr_str.clone(), tx.clone());
                        }
                        let _ = node_inner.clone().handle_incoming_message(msg, addr_str.clone()).await;
                    }
                }
                node_inner.peers.remove(&addr_str);
            }
        });
    }
}