use crate::engine::SentinelNode;
use anyhow::Result;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use sentinel_protocol::messages::{SentinelMessage, MessageContent};


pub async fn spawn_stdin_handler(node: Arc<SentinelNode>) -> Result<()> {
    let mut lines = BufReader::new(io::stdin()).lines();
    println!("READY TO CHAT. Type and hit Enter.");

    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        let msg = SentinelMessage::new(
            node.identity.node_id(),
            MessageContent::Chat(trimmed.to_string()),
        );

        let _ = node.persist_message(&msg);

        for peer in node.peers.iter() {
            let peer_state = peer.value();
            if let Err(e) = peer_state.tx.send(msg.clone()) {
                eprintln!("Failed to send to peer: {}", e);
            }
        }

        println!("[YOU]: {}", trimmed);
    }
    Ok(())
}