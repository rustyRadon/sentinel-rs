use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio_util::codec::Framed;
use futures::{SinkExt, StreamExt};
use uuid::Uuid;
use sentinel_protocol::{SentinelCodec, Frame};
use sentinel_transport::tls::TlsTransport;
use std::io::{self, Write};

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
    Chat { #[arg(short, long)] msg: String },
    Screenshot,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cli = Cli::parse();
    
    let client = sentinel_client::SentinelClient::new(&cli.ca_cert)?;
    let transport = client.connect(&cli.server, &cli.domain).await?;
    let mut framed = Framed::new(transport, SentinelCodec::new());

    framed.send(Frame::new(1, 0x02, "my_secure_password".into())?).await?;
    
    if let Some(result) = framed.next().await {
        let res = result?;
        if String::from_utf8_lossy(res.payload()) != "AUTH_OK" {
            println!("Authentication Failed!");
            return Ok(());
        }
    }

    match &cli.command {
        Commands::Chat { msg } => {
            let mut current_msg = msg.clone();
            loop {
                framed.send(Frame::new(1, 0x05, current_msg.into_bytes().into())?).await?;
                if let Some(result) = framed.next().await {
                    let res = result?;
                    println!("laptop> {}", String::from_utf8_lossy(res.payload()));
                }
                
                print!("phone> ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                current_msg = input.trim().to_string();
                if current_msg == "exit" { break; }
            }
        }
        _ => {
            handle_cmd(&cli, &mut framed).await?;
        }
    }
    Ok(())
}

async fn handle_cmd(cli: &Cli, framed: &mut Framed<TlsTransport, SentinelCodec>) -> anyhow::Result<()> {
    match &cli.command {
        Commands::Status => {
            framed.send(Frame::new(1, 0x01, vec![].into())?).await?;
        }
        Commands::Screenshot => {
            framed.send(Frame::new(1, 0x07, vec![].into())?).await?;
        }
        _ => {}
    }

    if let Some(result) = framed.next().await {
        let response = result?;
        process_response(cli, response)?;
    }
    Ok(())
}

fn process_response(cli: &Cli, response: Frame) -> anyhow::Result<()> {
    if response.flags() == 0x07 {
        let name = format!("snap_{}.png", Uuid::new_v4());
        std::fs::write(&name, response.payload())?;
        println!("Saved: {}", name);
    } else {
        println!("{}: {}", cli.server, String::from_utf8_lossy(response.payload()));
    }
    Ok(())
}