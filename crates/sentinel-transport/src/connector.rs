use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, client::TlsStream};
use rustls::{ClientConfig, RootCertStore, pki_types::ServerName};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use anyhow::{Result, Context};

pub struct SentinelConnector {
    config: Arc<ClientConfig>,
}

impl SentinelConnector {
    pub fn new(cert_path: &Path) -> Result<Self> {
        let mut root_store = RootCertStore::empty();
        
        // 1. Load native certificates
        let native_certs = rustls_native_certs::load_native_certs();
        for cert in native_certs.certs {
            root_store.add(cert)?;
        }
        
        // 2. Load our node certificate to trust peers in our network
        let cert_file = File::open(cert_path).context("Failed to open node.crt")?;
        let mut reader = BufReader::new(cert_file);
        let certs = rustls_pemfile::certs(&mut reader);
        for cert in certs {
            root_store.add(cert?)?;
        }

        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth(); 

        Ok(Self { config: Arc::new(config) })
    }

    pub async fn connect(&self, domain: &str, stream: TcpStream) -> Result<TlsStream<TcpStream>> {
        let connector = TlsConnector::from(self.config.clone());
        let server_name = ServerName::try_from(domain.to_string())
            .map_err(|_| anyhow::anyhow!("Invalid DNS Name"))?
            .to_owned();

        let tls_stream = connector.connect(server_name, stream).await?;
        Ok(tls_stream)
    }
}