use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_rustls::TlsAcceptor;
use tracing::{info, Level, error};
use tracing_subscriber;

mod engine;
mod router;
mod metrics;
mod handlers;

use crate::engine::SentinelEngine;
use crate::router::CommandRouter;
use crate::metrics::Metrics;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let cert_path = Path::new("certs/server.crt");
    let key_path = Path::new("certs/server.key");

    let certs = load_certs(cert_path).context("Failed to load server.crt")?;
    let key = load_private_key(key_path).context("Failed to load server.key")?;

    let tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("Invalid TLS certificate or private key")?;
    
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let addr = "0.0.0.0:8443";
    let listener = TcpListener::bind(addr).await.context("Failed to bind to port 8443")?;
    
    let router = Arc::new(CommandRouter::with_default_commands());
    let metrics = Metrics::new();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    info!("Sentinel Server starting on {}", addr);

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for shutdown signal");
        info!("Shutdown signal received, cleaning up...");
        let _ = shutdown_tx.send(());
    });

    let engine = SentinelEngine::new(
        listener, 
        acceptor, 
        router, 
        metrics, 
        shutdown_rx
    );

    if let Err(e) = engine.run().await {
        error!("Engine stopped with error: {:?}", e);
    }

    info!("Sentinel Server shut down successfully.");
    Ok(())
}

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    let file = File::open(path).context("Could not find certificate file")?;
    let mut reader = BufReader::new(file);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(certs)
}

fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
    let file = File::open(path).context("Could not find private key file")?;
    let mut reader = BufReader::new(file);
    
    if let Some(key_result) = rustls_pemfile::pkcs8_private_keys(&mut reader).next() {
        return Ok(PrivateKeyDer::Pkcs8(key_result?));
    }

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    if let Some(key_result) = rustls_pemfile::rsa_private_keys(&mut reader).next() {
        return Ok(PrivateKeyDer::Pkcs1(key_result?));
    }

    Err(anyhow::anyhow!("No valid private key (PKCS8 or RSA) found in {:?}", path))
}
