use anyhow::Result;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use crate::engine::SentinelNode;
use sentinel_protocol::messages::{MessageContent, SentinelMessage, SignalingMessage};

pub async fn handle_stdin(node: Arc<SentinelNode>) -> Result<()> {
    let mut reader = BufReader::new(io::stdin()).lines();

    while let Some(line) = reader.next_line().await? {
        let line = line.trim();
        if line.is_empty() { continue; }

        if line.starts_with('/') {
            let parts: Vec<&str> = line.split_whitespace().collect();
            match parts[0] {
                "/dial" => {
                    if parts.len() > 1 {
                        let target = parts[1].to_string();
                        //  if it contains a dot or colon, treat as direct IP dial
                        if target.contains('.') || target.contains(':') {
                            println!("Manual dial to address {}...", target);
                            let node_clone = Arc::clone(&node);
                            tokio::spawn(async move {
                                if let Err(e) = node_clone.dial_peer(target).await {
                                    eprintln!("Dial error: {}", e);
                                }
                            });
                        } else {
                            println!("Requesting lookup for Node ID: {}...", target);
                            let lookup_msg = SentinelMessage::new_signal(
                                node.identity.node_id(),
                                SignalingMessage::LookupRequest { target_id: target }
                            );
                            let _ = node.signaler_tx.send(lookup_msg);
                        }
                    } else {
                        println!("Usage: /dial <address:port> OR /dial <node_id>");
                    }
                }
                "/peers" => {
                    println!("--- Connected Peers ----");
                    if node.peers.is_empty() {
                        println!("No active peer connections.");
                    } else {
                        for entry in node.peers.iter() {
                            println!("ADDR: {} | ID: {} | NAME: {}", 
                                entry.key(), 
                                entry.value().node_id, 
                                entry.value().node_name
                            );
                        }
                    }
                }
                "/history" => {
                    println!("--- Local Message History (Last 10) ---");
                    if let Err(e) = node.print_history() {
                        eprintln!("Error reading history: {}", e);
                    }
                }
                "/id" => {
                    println!("YOUR NODE ID: {}", node.identity.node_id());
                }
                _ => println!("Unknown command. Available: /dial, /peers, /history, /id"),
            }
        } else {
            let content = MessageContent::Chat(line.to_string());
            for entry in node.peers.iter() {
                node.sign_and_send(&entry.value().tx, SentinelMessage::new(
                    node.identity.node_id(),
                    content.clone(),
                ));
            }
            println!("[YOU]: {}", line);
        }
    }
    Ok(())
}