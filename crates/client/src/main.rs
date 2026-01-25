use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio_util::codec::Framed;
use futures::{SinkExt, StreamExt};
use uuid::Uuid;
use sentinel_client::SentinelClient;
use sentinel_protocol::{SentinelCodec, Frame};

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "10.114.101.7:8443")]
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
    Chat { #[arg(short, long)] msg: String },
    Screenshot,
    Shutdown,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let client = SentinelClient::new(&cli.ca_cert)?;
    let transport = client.connect(&cli.server, &cli.domain).await?;
    let mut framed = Framed::new(transport, SentinelCodec::new());

    match cli.command {
        Commands::Status => {
            framed.send(Frame::new(1, 0x01, vec![].into())?).await?;
        }
        Commands::Upload { path } => {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("bin");
            let data = std::fs::read(&path)?;
            let mut payload = ext.as_bytes().to_vec();
            payload.push(b'|');
            payload.extend_from_slice(&data);
            framed.send(Frame::new(1, 0x03, payload.into())?).await?;
        }
        Commands::Chat { msg } => {
            framed.send(Frame::new(1, 0x05, msg.into_bytes().into())?).await?;
        }
        Commands::Screenshot => {
            framed.send(Frame::new(1, 0x07, vec![].into())?).await?;
        }
        Commands::Shutdown => {
            framed.send(Frame::new(1, 0x99, vec![].into())?).await?;
        }
    }

    if let Some(Ok(response)) = framed.next().await {
        if response.flags() == 0x07 && response.payload().len() > 100 {
            let name = format!("remote_snap_{}.png", Uuid::new_v4());
            std::fs::write(&name, response.payload())?;
            println!("SUCCESS: Screenshot saved as {}", name);
        } else {
            println!("SERVER_RESPONSE: {}", String::from_utf8_lossy(response.payload()));
        }
    }
    Ok(())
}
