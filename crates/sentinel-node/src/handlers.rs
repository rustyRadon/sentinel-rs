use anyhow::Result;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use crate::engine::SentinelNode;
use sentinel_protocol::messages::MessageContent;

pub async fn handle_stdin(node: Arc<SentinelNode>) -> Result<()> {
    let mut reader = BufReader::new(io::stdin()).lines();

    while let Some(line) = reader.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('/') {
            let parts: Vec<&str> = line.split_whitespace().collect();
            match parts[0] {
                "/dial" => {
                    if parts.len() > 1 {
                        let addr = parts[1].to_string();
                        println!("Attempting to dial {}...", addr);
                        let node_clone = Arc::clone(&node);
                        tokio::spawn(async move {
                            if let Err(e) = node_clone.dial_peer(addr).await {
                                eprintln!("Dial error: {}", e);
                            }
                        });
                    } else {
                        println!("Usage: /dial <address:port>");
                    }
                }
                "/peers" => {
                    println!("--- Connected Peers ---");
                    for entry in node.peers.iter() {
                        println!("{} | ID: {} | Name: {}", entry.key(), entry.value().node_id, entry.value().node_name);
                    }
                }
                "/history" => {
                    let _ = node.print_history();
                }
                _ => println!("Unknown command. Try /dial, /peers, or /history"),
            }
        } else {
            let content = MessageContent::Chat(line.to_string());
            
            // send to everyone we know hmmmmmm gotta check back
            for entry in node.peers.iter() {
                node.sign_and_send(&entry.value().tx, sentinel_protocol::messages::SentinelMessage::new(
                    node.identity.node_id(),
                    content.clone(),
                ));
            }
            println!("[YOU]: {}", line);
        }
    }
    Ok(())
}