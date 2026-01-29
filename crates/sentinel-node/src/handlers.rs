use crate::engine::SentinelNode;
use anyhow::Result;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use sentinel_protocol::messages::{SentinelMessage, MessageContent};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn spawn_stdin_handler(node: Arc<SentinelNode>) -> Result<()> {
    let mut lines = BufReader::new(io::stdin()).lines();
    println!("READY TO CHAT. Type and hit Enter.");

    while let Ok(Some(line)) = lines.next_line().await {
        let msg = SentinelMessage {
            id: Uuid::new_v4(),
            sender: node.identity.node_id(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            content: MessageContent::Chat(line.clone()),
        };

        node.persist_message(&msg)?;

        for peer in node.peers.iter() {
            let sender = peer.value();
            if let Err(_) = sender.send(msg.clone()) {
            }
        }

        println!("[YOU]: {}", line);
    }
    Ok(())
}