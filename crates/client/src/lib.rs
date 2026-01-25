use std::sync::Arc;
use std::path::Path;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use rustls::pki_types::ServerName;
use anyhow::Result;

use sentinel_transport::tls_config::load_certs;
use sentinel_transport::TlsTransport;

pub struct SentinelClient {
    connector: TlsConnector,
}

impl SentinelClient {
    pub fn new(ca_path: &Path) -> Result<Self> {
        let mut root_cert_store = RootCertStore::empty();
        let ca_certs = load_certs(ca_path)?;
        
        for cert in ca_certs {
            root_cert_store.add(cert)?;
        }

        let config = ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        Ok(Self {
            connector: TlsConnector::from(Arc::new(config)),
        })
    }

    pub async fn connect(&self, addr: &str, domain: &str) -> Result<TlsTransport<TcpStream>> {
        let server_name = ServerName::try_from(domain.to_string())
            .map_err(|_| anyhow::anyhow!("Invalid server name"))?
            .to_owned();

        let stream = TcpStream::connect(addr).await?;
        let tls_stream = self.connector.connect(server_name, stream).await?;
        
        Ok(TlsTransport::new(tls_stream.into()))
    }
}