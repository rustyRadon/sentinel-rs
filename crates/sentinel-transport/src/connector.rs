use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, client::TlsStream};
use rustls::{ClientConfig, pki_types::ServerName};
use anyhow::Result;

//  Custom Verifier
#[derive(Debug)]
struct DangerVerifier;

impl rustls::client::danger::ServerCertVerifier for DangerVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>, 
        _intermediates: &[rustls::pki_types::CertificateDer<'_>], 
        _server_name: &ServerName,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>, 
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        use rustls::SignatureScheme::*;
        vec![
            ECDSA_NISTP256_SHA256,
            ECDSA_NISTP384_SHA384,
            RSA_PSS_SHA256,
            RSA_PSS_SHA384,
            RSA_PSS_SHA512,
            ED25519, 
        ]
    }
}

// connector 
pub struct SentinelConnector {
    config: Arc<ClientConfig>,
}

impl SentinelConnector {
    pub fn new() -> Self {
        let config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(DangerVerifier))
            .with_no_client_auth();

        Self { config: Arc::new(config) }
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