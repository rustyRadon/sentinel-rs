use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use clap::Parser;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::Framed;
use tracing::{info, error, debug};

use sentinel_transport::SentinelAcceptor;
use sentinel_protocol::SentinelCodec;
use crate::router::CommandRouter;

mod router;
mod handlers;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0:8443")]
    addr: String,
    #[arg(long, default_value = "certs/server.crt")]
    cert: PathBuf,
    #[arg(long, default_value = "certs/server.key")]
    key: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let router = Arc::new(CommandRouter::with_default_commands());

    let acceptor = SentinelAcceptor::new(&args.cert, &args.key, Duration::from_secs(10))?;
    let listener = TcpListener::bind(&args.addr).await?;
    
    println!("SENTINEL_BASE_ACTIVE: Listening on {}", args.addr);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let router = Arc::clone(&router);

        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(transport) => {
                    info!("Connection secured: {}", peer_addr);
                    let mut framed = Framed::new(transport, SentinelCodec::new());

                    while let Some(result) = framed.next().await {
                        match result {
                            Ok(frame) => {
                                let cmd = frame.flags();
                                
                                if cmd == 0x99 {
                                    println!("[SYSTEM] Shutdown signal from {}", peer_addr);
                                    std::process::exit(0);
                                }

                                match router.dispatch(frame).await {
                                    Ok(Some(response)) => {
                                        if let Err(e) = framed.send(response).await {
                                            error!("Failed to send response: {}", e);
                                        }
                                    }
                                    Ok(None) => debug!("Command handled (fire-and-forget)"),
                                    Err(e) => error!("Routing error: {}", e),
                                }
                            }
                            Err(e) => {
                                error!("Protocol error from {}: {}", peer_addr, e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => error!("TLS Handshake failed for {}: {}", peer_addr, e),
            }
        });
    }
}