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
    // 1. Initialize Crypto Provider
    rustls::crypto::aws_lc_rs::default_provider().install_default().ok();

    // 2. Initialize Node State
    let node = Arc::new(SentinelNode::new(PathBuf::from("./.sentinel")).await?);
    
    // 3. UI/Startup tasks
    node.print_history()?;
    node.start_discovery(8443)?;

    let stdin_node = Arc::clone(&node);
    tokio::spawn(async move {
        if let Err(e) = handlers::spawn_stdin_handler(stdin_node).await {
            eprintln!("Keyboard error: {}", e);
        }
    });

    // 4. Server Loop (Passive Listening)
    let listener = TcpListener::bind("0.0.0.0:8443").await?;
    println!("LISTENING ON 8443...");

    loop {
        let (stream, _) = listener.accept().await?;
        let acceptor = node.acceptor.clone();
        let node_inner = Arc::clone(&node);

        tokio::spawn(async move {
            if let Ok(tls) = acceptor.accept(stream).await {
                let (mut sink, mut stream) = Framed::new(tls, SentinelCodec::new()).split();
                
                let (tx, mut rx) = mpsc::unbounded_channel::<SentinelMessage>();
                let mut peer_id = String::from("unknown");

                // Task for SENDING messages to this specific connection
                let send_task = tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        if let Ok(frame) = Frame::new(1, 0, bytes::Bytes::from(msg.to_bytes())) {
                            if let Err(e) = sink.send(frame).await {
                                eprintln!("Failed to send to peer: {}", e);
                                break;
                            }
                        }
                    }
                });

                // Loop for RECEIVING messages from this specific connection
                while let Some(Ok(frame)) = stream.next().await {
                    if let Ok(msg) = SentinelMessage::from_bytes(frame.payload()) {
                        if peer_id == "unknown" {
                            peer_id = msg.sender.clone();
                            node_inner.peers.insert(peer_id.clone(), tx.clone());
                            println!("Peer identified: {}", peer_id);
                        }

                        // {:?} used here to fix formatting error
                        println!("[{}] {:?}", msg.sender, msg.content);
                        let _ = node_inner.persist_message(&msg);
                    }
                }
                
                println!("Peer disconnected: {}", peer_id);
                node_inner.peers.remove(&peer_id);
                send_task.abort();
            }
        });
    }
}