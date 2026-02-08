use anyhow::Result;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use sentinel_protocol::{MessageContent, SentinelCodec, SentinelMessage, SignalingMessage};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;

type PeerDirectory = Arc<DashMap<String, SocketAddr>>;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let addr = "0.0.0.0:8888";
    let listener = TcpListener::bind(addr).await?;
    let directory: PeerDirectory = Arc::new(DashMap::new());

    println!("SENTINEL SIGNALER live on {}", addr);

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        let directory_ref = Arc::clone(&directory);

        tokio::spawn(async move {
            if let Err(e) = handle_signaling_node(directory_ref, socket, peer_addr).await {
                eprintln!("Signaler error for {}: {:?}", peer_addr, e);
            }
        });
    }
}

async fn handle_signaling_node(
    dir: PeerDirectory,
    socket: TcpStream,
    peer_addr: SocketAddr,
) -> Result<()> {
    let mut framed = Framed::new(socket, SentinelCodec::new());

    if let Some(Ok(msg)) = framed.next().await {
        if let MessageContent::Signal(SignalingMessage::Register { node_id, .. }) = msg.content {
            println!("Node {} registered from {}", node_id, peer_addr);
            
            dir.insert(node_id.clone(), peer_addr);
            
            while let Some(result) = framed.next().await {
                match result {
                    Ok(client_msg) => {
                        process_signal(&dir, &mut framed, client_msg, &node_id).await?;
                    }
                    Err(e) => {
                        eprintln!("Connection lost for {}: {}", node_id, e);
                        break;
                    }
                }
            }

            dir.remove(&node_id);
            println!("Node {} deregistered", node_id);
        }
    }

    Ok(())
}

async fn process_signal(
    dir: &PeerDirectory,
    framed: &mut Framed<TcpStream, SentinelCodec>,
    msg: SentinelMessage,
    sender_id: &str,
) -> Result<()> {
    if let MessageContent::Signal(signal) = msg.content {
        match signal {
            SignalingMessage::LookupRequest { target_id } => {
                if let Some(target_addr) = dir.get(&target_id) {
                    let response = SentinelMessage::new_signal(
                        sender_id.to_string(),
                        SignalingMessage::PeerResponse {
                            peer_id: target_id,
                            public_addr: *target_addr,
                        },
                    );
                    framed.send(response).await?;
                } else {
                    let err = SentinelMessage::new_signal(
                        sender_id.to_string(),
                        SignalingMessage::Error("Peer not found".to_string()),
                    );
                    framed.send(err).await?;
                }
            }
            _ => println!("Signal not yet implemented: {:?}", signal),
        }
    }
    Ok(())
}