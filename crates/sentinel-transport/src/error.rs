use thiserror::Error;

pub type TransportResult<T> = Result<T, TransportError>;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("TLS error: {0}")]
    Tls(#[from] std::io::Error),
    
    #[error("Handshake timed out")]
    HandshakeTimeout,
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Handshake failed")]
    HandshakeFailed,
}