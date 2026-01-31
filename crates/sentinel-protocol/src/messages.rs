use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::net::SocketAddr;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerInfo {
    pub node_id: String,    
    pub address: SocketAddr, 
    pub node_name: String,
    pub last_seen: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageContent {
    Chat(String),
    Handshake { 
        public_key: Vec<u8>,
        node_name: String 
    },
    PeerDiscovery(Vec<PeerInfo>), 
    
    Ping,
    Pong,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SentinelMessage {
    pub id: Uuid,           
    pub sender: String,     
    pub timestamp: u64,     
    pub content: MessageContent,
}

impl SentinelMessage {
    pub fn new(sender: String, content: MessageContent) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            content,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Serialization failed")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}