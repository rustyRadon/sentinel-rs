use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use std::net::SocketAddr;

#[async_trait]
pub trait SentinelTransport: AsyncRead + AsyncWrite + Unpin + Send {
    /// returns the remote address of the peer.
    fn peer_addr(&self) -> Result<SocketAddr, std::io::Error>;

    /// returns true if the transport is encrypted (TLS).
    fn is_secure(&self) -> bool;
}
