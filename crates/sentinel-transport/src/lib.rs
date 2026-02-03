pub mod tcp;
pub mod tls;
pub mod tls_config;
pub mod acceptor;
pub mod error;
pub mod metrics;
pub mod state;
pub mod connector;

pub use acceptor::SentinelAcceptor;
pub use error::{TransportError, TransportResult};
pub use tcp::RawTcpTransport;
pub use tls::TlsTransport;
pub use state::{Connection, Unauthenticated};
pub use connector::SentinelConnector;

use tokio::io::{AsyncRead, AsyncWrite};
use std::net::SocketAddr;

/// The base trait for all Sentinel network communications.
pub trait SentinelTransport: AsyncRead + AsyncWrite + Unpin + Send {
    /// Returns the remote address of the peer.
    fn peer_addr(&self) -> Result<SocketAddr, std::io::Error>;

    /// Returns true if the transport is encrypted (TLS).
    fn is_secure(&self) -> bool;
}