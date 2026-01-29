use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageContent {
    Chat(String),
    Handshake { 
        public_key: Vec<u8>,
        node_name: String 
    },
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
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Serialization failed")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}