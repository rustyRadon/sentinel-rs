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
pub enum SignalingMessage {
    /// Node -> Signaler: Register with public key and signature of a nonce
    Register {
        node_id: String,
        public_key: Vec<u8>,
        signature: Vec<u8>,
    },
    /// Node -> Signaler: Ask for a peer's public address
    LookupRequest {
        target_id: String,
    },
    /// Signaler -> Node: Metadata for the target peer
    PeerResponse {
        peer_id: String,
        public_addr: SocketAddr,
    },
    /// Signaler -> Both Nodes: Trigger the simultaneous open
    PunchCommand {
        target_addr: SocketAddr,
        /// Unix timestamp (nanos) for synchronized connection start
        timestamp_ns: u64,
    },
    /// Error reporting for signaling failures
    Error(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageContent {
    Chat(String),
    Handshake { 
        public_key: Vec<u8>,
        node_name: String 
    },
    PeerDiscovery(Vec<PeerInfo>),
    Signal(SignalingMessage),
    Ping,
    Pong,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SentinelMessage {
    pub id: Uuid,           
    pub sender: String,  
    pub public_key: Vec<u8>,   
    pub timestamp: u64,     
    pub content: MessageContent,
    pub signature: Vec<u8>,
}

impl SentinelMessage {
    pub fn new(sender: String, content: MessageContent) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender,
            public_key: vec![],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            content,
            signature: vec![],
        }
    }

    /// Convenience helper for creating signaling messages
    pub fn new_signal(sender: String, signal: SignalingMessage) -> Self {
        Self::new(sender, MessageContent::Signal(signal))
    }

    pub fn sig_hash(&self) -> Vec<u8> {
        let mut data = self.id.as_bytes().to_vec();
        data.extend_from_slice(self.sender.as_bytes());
        data.extend_from_slice(&self.timestamp.to_le_bytes());
        data.extend_from_slice(&bincode::serialize(&self.content).unwrap_or_default());
        data
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Serialization failed")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}