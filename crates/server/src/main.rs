use std::path::PathBuf;
use std::time::Duration;
use std::sync::{Arc, Mutex}; 
use clap::Parser;
use futures::{SinkExt, StreamExt};
use tokio_util::codec::Framed;
use tracing::{info, error};
use sysinfo::System;
use uuid::Uuid;
use screenshots::Screen;

use sentinel_transport::SentinelAcceptor;
use sentinel_protocol::{SentinelCodec, Frame};

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
    let sys = Arc::new(Mutex::new(System::new_all()));
    let acceptor = SentinelAcceptor::new(&args.cert, &args.key, Duration::from_secs(10))?;
    let listener = tokio::net::TcpListener::bind(&args.addr).await?;
    
    println!("SENTINEL_BASE_ACTIVE: Listening on {}", args.addr);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let sys_handle = Arc::clone(&sys); 

        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(transport) => {
                    let mut framed = Framed::new(transport, SentinelCodec::new());

                    while let Some(result) = framed.next().await {
                        match result {
                            Ok(request) => {
                                let cmd = request.flags();
                                let mut should_exit = false;

                                let payload = match cmd {
                                    0x01 => {
                                        let mut s = sys_handle.lock().unwrap();
                                        s.refresh_cpu();
                                        s.refresh_memory();
                                        let load = s.global_cpu_info().cpu_usage();
                                        let total_mem = s.total_memory() / 1024 / 1024;
                                        let used_mem = s.used_memory() / 1024 / 1024;
                                        format!("CPU Load: {:.1}% | Mem: {}/{} MB", load, used_mem, total_mem).into_bytes()
                                    },
                                    0x03 => {
                                        let full_payload = request.payload();
                                        if let Some(pos) = full_payload.iter().position(|&b| b == b'|') {
                                            let ext = String::from_utf8_lossy(&full_payload[..pos]);
                                            let data = &full_payload[pos + 1..];
                                            let _ = std::fs::create_dir_all("uploads");
                                            let file_id = Uuid::new_v4().to_string();
                                            let path = format!("uploads/{}.{}", file_id, ext);
                                            match std::fs::write(&path, data) {
                                                Ok(_) => format!("Saved as {}.{}", file_id, ext).into_bytes(),
                                                Err(_) => b"Disk Error".to_vec(),
                                            }
                                        } else { b"Invalid Upload Format".to_vec() }
                                    },
                                    0x05 => {
                                        let msg = String::from_utf8_lossy(request.payload());
                                        println!("[REMOTE_CHAT] {}: {}", peer_addr, msg);
                                        b"Message received".to_vec()
                                    },
                                    0x07 => {
                                        println!("[SYSTEM] Capturing screenshot for {}", peer_addr);
                                        match Screen::all().unwrap().first() {
                                            Some(screen) => {
                                                let image = screen.capture().unwrap();
                                                let buffer = image.to_png().unwrap();
                                                let file_id = Uuid::new_v4().to_string();
                                                let path = format!("uploads/snap_{}.png", file_id);
                                                let _ = std::fs::create_dir_all("uploads");
                                                let _ = std::fs::write(&path, &buffer);
                                                println!("[SYSTEM] Saved locally to {}", path);
                                                buffer
                                            },
                                            None => b"Error: Screen capture failed".to_vec(),
                                        }
                                    },
                                    0x99 => {
                                        println!("[SYSTEM] Shutdown signal from {}", peer_addr);
                                        should_exit = true;
                                        b"Server shutting down...".to_vec()
                                    },
                                    _ => b"Unknown Command".to_vec(),
                                };

                                let response = Frame::new(1, cmd, payload.into()).unwrap();
                                let _ = framed.send(response).await;

                                if should_exit {
                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                    std::process::exit(0);
                                }
                            }
                            Err(e) => { error!("Protocol error: {}", e); break; }
                        }
                    }
                }
                Err(e) => error!("Handshake failed: {}", e),
            }
        });
    }
}