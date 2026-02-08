use anyhow::Result;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use sentinel_protocol::{MessageContent, SentinelCodec, SentinelMessage, SignalingMessage};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;

// Maps NodeID -> Current Public Address
type PeerDirectory = Arc<DashMap<String, SocketAddr>>;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let addr = "0.0.0.0:8888";
    let listener = TcpListener::bind(addr).await?;
    let directory: PeerDirectory = Arc::new(DashMap::new());

    println!("SENTINEL SIGNALER live on {}", addr);

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        let directory_ref = Arc::clone(&directory);

        tokio::spawn(async move {
            if let Err(e) = handle_signaling_node(directory_ref, socket, peer_addr).await {
                eprintln!("Error handling node {}: {:?}", peer_addr, e);
            }
        });
    }
}

async fn handle_signaling_node(
    dir: PeerDirectory,
    socket: TcpStream,
    peer_addr: SocketAddr,
) -> Result<()> {
    // 1. Wrap the socket with our SentinelCodec for framed messaging
    let mut framed = Framed::new(socket, SentinelCodec::new());

    // 2. Wait for the Registration message
    // Note: In a production build, we would send a Challenge here first.
    if let Some(Ok(msg)) = framed.next().await {
        if let MessageContent::Signal(SignalingMessage::Register { node_id, .. }) = msg.content {
            println!("Node {} registered from {}", node_id, peer_addr);
            
            // Insert into directory
            dir.insert(node_id.clone(), peer_addr);
            
            // 3. Enter the main control loop
            while let Some(result) = framed.next().await {
                match result {
                    Ok(client_msg) => {
                        handle_client_request(&dir, &mut framed, client_msg, &node_id).await?;
                    }
                    Err(e) => {
                        eprintln!("Protocol error from {}: {}", node_id, e);
                        break;
                    }
                }
            }

            // Cleanup on disconnect
            dir.remove(&node_id);
            println!("Node {} disconnected", node_id);
        }
    }

    Ok(())
}

async fn handle_client_request(
    dir: &PeerDirectory,
    framed: &mut Framed<TcpStream, SentinelCodec>,
    msg: SentinelMessage,
    sender_id: &str,
) -> Result<()> {
    match msg.content {
        MessageContent::Signal(SignalingMessage::LookupRequest { target_id }) => {
            if let Some(target_addr) = dir.get(&target_id) {
                // Found the peer! Send their public address back to the caller
                let response = SentinelMessage::new_signal(
                    sender_id.to_string(),
                    SignalingMessage::PeerResponse {
                        peer_id: target_id,
                        public_addr: *target_addr,
                    },
                );
                framed.send(response).await?;
            } else {
                // Peer not found
                let err = SentinelMessage::new_signal(
                    sender_id.to_string(),
                    SignalingMessage::Error("Peer not found in directory".to_string()),
                );
                framed.send(err).await?;
            }
        }
        _ => {
            // Log unhandled signaling messages
            println!("Unhandled signal from {}: {:?}", sender_id, msg.content);
        }
    }
    Ok(())
}