use std::sync::atomic::{AtomicU64, Ordering};

pub struct TransportMetrics {
    pub total_connections: AtomicU64,
    pub active_connections: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub handshakes_failed: AtomicU64,
}

impl TransportMetrics {
    pub fn connection_started(&self) {
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }
}