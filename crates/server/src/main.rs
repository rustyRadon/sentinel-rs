use std::path::PathBuf;
use std::time::Duration;
use std::sync::{Arc, Mutex}; 
use clap::Parser;
use futures::{SinkExt, StreamExt};
use tokio_util::codec::Framed;
use tracing::{info, error};
use sysinfo::System;

use sentinel_transport::SentinelAcceptor;
use sentinel_protocol::{SentinelCodec, Frame};

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1:8443")]
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

    let sys = Arc::new(Mutex::new(System::new_all()));

    let acceptor = SentinelAcceptor::new(
        &args.cert,
        &args.key,
        Duration::from_secs(10),
    )?;

    let listener = tokio::net::TcpListener::bind(&args.addr).await?;
    info!("Sentinel Server listening on {}", args.addr);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let sys_handle = Arc::clone(&sys); 

        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(transport) => {
                    info!("Connection secured: {}", peer_addr);
                    let mut framed = Framed::new(transport, SentinelCodec::new());

                    while let Some(result) = framed.next().await {
                        match result {
                            Ok(request) => {
                                let cmd = request.flags();
                                
                                let payload = match cmd {
                                    0x01 => {
                                        let mut s = sys_handle.lock().unwrap();
                                        s.refresh_cpu();
                                        s.refresh_memory();

                                        let load = s.global_cpu_info().cpu_usage();
                                        let total_mem = s.total_memory() / 1024 / 1024;
                                        let used_mem = s.used_memory() / 1024 / 1024;
                                        
                                        format!(
                                            "CPU Load: {:.1}% | Mem: {}/{} MB", 
                                            load, used_mem, total_mem
                                        ).into_bytes()
                                    },
                                    0x03 => b"File upload received".to_vec(),
                                    _ => b"Unknown command".to_vec(),
                                };

                                let response = Frame::new(1, cmd, payload.into()).unwrap();
                                if let Err(e) = framed.send(response).await {
                                    error!("Send error: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Protocol error: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => error!("Handshake failed for {}: {}", peer_addr, e),
            }
        });
    }
}