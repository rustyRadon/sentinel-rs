use thiserror::Error;

#[derive(Error, Debug)]
pub enum SentinelTransportError {
    #[error("IO failure: {0}")]
    Io(#[from] std::io::Error),

    #[error("TLS Handshake failed: {0}")]
    Tls(#[from] tokio_rustls::rustls::Error),

    #[error("Handshake timed out after {0} seconds")]
    Timeout(u64),

    #[error("Invalid certificate: {0}")]
    InvalidCertificate(String),
}