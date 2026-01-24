use std::path::Path;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use futures::{SinkExt, StreamExt};

use sentinel_transport::acceptor::SentinelAcceptor; 
use sentinel_protocol::codec::SentinelCodec;
use sentinel_protocol::frame::Frame;

pub struct SentinelClient {
    framed: Framed<tokio_rustls::client::TlsStream<TcpStream>, SentinelCodec>,
}

impl SentinelClient {
    /// Connects to a Sentinel Server with full TLS 1.3 verification
    pub async fn connect(
        addr: &str,
        ca_path: &Path,
        domain: &str,
        timeout: Duration,
    ) -> anyhow::Result<Self> {
        let stream = tokio::time::timeout(timeout, TcpStream::connect(addr)).await??;
        
        let mut root_cert_store = tokio_rustls::rustls::RootCertStore::empty();
        let ca_certs = sentinel_transport::tls_config::load_certs(ca_path)?;
        for cert in ca_certs {
            root_cert_store.add(cert.0[0].clone().into())?;
        }

        let config = tokio_rustls::rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth(); 

        let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
        let domain = tokio_rustls::rustls::ServerName::try_from(domain)
            .map_err(|_| anyhow::anyhow!("Invalid DNS name"))?;

        let tls_stream = connector.connect(domain, stream).await?;

        let framed = Framed::new(tls_stream, SentinelCodec::new());

        Ok(Self { framed })
    }

    pub async fn send_frame(&mut self, frame: Frame) -> anyhow::Result<()> {
        self.framed.send(frame).await.map_err(Into::into)
    }

    pub async fn next_frame(&mut self) -> anyhow::Result<Option<Frame>> {
        self.framed.next().await.transpose().map_err(Into::into)
    }
}