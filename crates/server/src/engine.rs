use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_util::codec::Framed;
use futures::{StreamExt, SinkExt};
use tokio::sync::oneshot;
use tracing::{info, warn};

use sentinel_protocol::{Frame, codec::SentinelCodec};
use crate::router::CommandRouter;
use crate::metrics::Metrics;

pub struct SentinelEngine {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    router: Arc<CommandRouter>,
    metrics: Metrics,
    shutdown_rx: oneshot::Receiver<()>,
}

impl SentinelEngine {
    pub fn new(
        listener: TcpListener, 
        acceptor: TlsAcceptor, 
        router: Arc<CommandRouter>,
        metrics: Metrics,
        shutdown_rx: oneshot::Receiver<()>
    ) -> Self {
        Self { listener, acceptor, router, metrics, shutdown_rx }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("Sentinel Engine loop started");

        loop {
            tokio::select! {
                accept_result = self.listener.accept() => {
                    let (stream, peer_addr) = accept_result?;
                    let acceptor = self.acceptor.clone();
                    let router = self.router.clone();
                    let metrics = self.metrics.clone();

                    tokio::spawn(async move {
                        metrics.increment_connections();
                        info!("New connection from {}", peer_addr);
                        
                        if let Err(e) = Self::handle_client(stream, acceptor, router, peer_addr).await {
                            warn!("[{}] Connection closed: {:?}", peer_addr, e);
                        }
                        
                        metrics.decrement_connections();
                    });
                }

                _ = &mut self.shutdown_rx => {
                    info!("Engine received shutdown; stopping listener...");
                    break;
                }
            }
        }
        Ok(())
    }

    async fn handle_client(
        stream: tokio::net::TcpStream,
        acceptor: TlsAcceptor,
        router: Arc<CommandRouter>,
        peer_addr: std::net::SocketAddr,
    ) -> anyhow::Result<()> {
        let tls_stream = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            acceptor.accept(stream)
        ).await??;

        let mut framed = Framed::new(tls_stream, SentinelCodec::new());
        let mut authenticated_user: Option<String> = None;

        while let Some(result) = framed.next().await {
            let frame = result?;

            if authenticated_user.is_none() {
                if frame.flags() == 0x02 {
                    let password = String::from_utf8_lossy(frame.payload());
                    if password == "my_secure_password" {
                        authenticated_user = Some("Admin".to_string());
                        info!("[{}] User authenticated as Admin", peer_addr);
                        let ok_frame = Frame::new(1, 0x02, "AUTH_OK".into())?;
                        framed.send(ok_frame).await?;
                    } else {
                        warn!("[{}] Failed login attempt", peer_addr);
                        let fail_frame = Frame::new(1, 0x02, "AUTH_FAILED".into())?;
                        framed.send(fail_frame).await?;
                    }
                } else {
                    // Reject any other command if not logged in
                    let err_frame = Frame::new(1, 0x00, "ERR: Authenticate first".into())?;
                    framed.send(err_frame).await?;
                }
                continue;
            }

            if let Some(response) = router.dispatch(frame).await? {
                framed.send(response).await?;
            }
        }
        
        info!("[{}] Client disconnected", peer_addr);
        Ok(())
    }
}