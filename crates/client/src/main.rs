use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio_util::codec::Framed;
use futures::{SinkExt, StreamExt};

use sentinel_client::SentinelClient;
use sentinel_protocol::{SentinelCodec, Frame};

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "127.0.0.1:8443")]
    server: String,
    #[arg(short, long, default_value = "localhost")]
    domain: String,
    #[arg(short, long, default_value = "certs/ca.crt")]
    ca_cert: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Status,
    Upload { #[arg(short, long)] path: PathBuf },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let client = SentinelClient::new(&cli.ca_cert)?;
    let transport = client.connect(&cli.server, &cli.domain).await?;
    
    let mut framed = Framed::new(transport, SentinelCodec::new());

    match cli.command {
        Commands::Status => {
            let request = Frame::new(1, 0x01, vec![].into())?;
            framed.send(request).await?;

            if let Some(Ok(response)) = framed.next().await {
                println!(">>> Server Status: {}", String::from_utf8_lossy(response.payload()));
            }
        }
        Commands::Upload { path } => {
            let data = std::fs::read(path)?;
            let request = Frame::new(1, 0x03, data.into())?;
            framed.send(request).await?;

            if let Some(Ok(response)) = framed.next().await {
                println!(">>> Server Response: {}", String::from_utf8_lossy(response.payload()));
            }
        }
    }

    Ok(())
}