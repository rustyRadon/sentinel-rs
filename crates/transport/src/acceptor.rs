use std::sync::Arc;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use crate::tls_config::{load_certs, load_private_key};
use std::path::Path;

pub struct SentinelAcceptor {
    acceptor: TlsAcceptor,
}

impl SentinelAcceptor {
    pub fn new(cert_path: &Path, key_path: &Path) -> anyhow::Result<Self> {
        let certs = load_certs(cert_path)?;
        let key = load_private_key(key_path)?;

        let mut config = ServerConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_safe_default_protocol_versions()?
            .with_no_client_auth() 
            .with_single_cert(certs, key)?;

        config.alpn_protocols = vec![b"sentinel-v1".to_vec()];

        Ok(Self {
            acceptor: TlsAcceptor::from(Arc::new(config)),
        })
    }
}