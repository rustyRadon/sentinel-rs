use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Invalid magic bytes in header")]
    InvalidMagic,

    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(u8),

    #[error("Frame payload size exceeds maximum limit")]
    FrameTooLarge,

    #[error("Frame received with zero length payload")]
    ZeroLengthFrame,

    #[error("CRC32 integrity check failed")]
    IntegrityCheckFailed,

    #[error("Incomplete frame data")]
    Incomplete,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol serialization error: {0}")]
    SerializationError(String),

}