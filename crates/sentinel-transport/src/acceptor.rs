use std::sync::Arc;
use std::path::Path;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;

use crate::tls::TlsTransport;
use crate::tls_config::{load_certs, load_private_key};
use crate::error::{TransportError, TransportResult};

#[derive(Clone)]
pub struct SentinelAcceptor {
    inner: TlsAcceptor,
    handshake_timeout: Duration,
}

impl SentinelAcceptor {
    pub fn new(
        cert_path: &Path,
        key_path: &Path,
        handshake_timeout: Duration,
    ) -> anyhow::Result<Self> {
        let certs = load_certs(cert_path)?;
        let key = load_private_key(key_path)?;

        let mut config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        config.alpn_protocols = vec![b"sentinel-v1".to_vec()];

        Ok(Self {
            inner: TlsAcceptor::from(Arc::new(config)),
            handshake_timeout,
        })
    }

    pub async fn accept(&self, stream: TcpStream) -> TransportResult<TlsTransport<TcpStream>> {
        let handshake_future = self.inner.accept(stream);
        
        match tokio::time::timeout(self.handshake_timeout, handshake_future).await {
            Ok(result) => {
                let tls_stream = result.map_err(TransportError::Tls)?;
                Ok(TlsTransport::new(tls_stream.into()))
            }
            Err(_) => Err(TransportError::HandshakeTimeout),
        }
    }
}