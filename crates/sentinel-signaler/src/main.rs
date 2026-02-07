use tokio::net::TcpListener;
use std::sync::Arc;
use dashmap::DashMap;
use std::net::SocketAddr;
use sentinel_protocol::messages;

type PeerDirectory = Arc<DashMap<String, SocketAddr>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let addr = "0.0.0.0:8888";
    let listener = TcpListener::bind(addr).await?;
    let directory: PeerDirectory = Arc::new(DashMap::new());

    println!("SENTINEL SIGNALER live on {}", addr);

    loop {
        let (mut _socket, peer_addr) = listener.accept().await?;
        let directory_ref = Arc::clone(&directory);

        tokio::spawn(async move {
            println!("Node connected from {}", peer_addr);
            
            // handling the registration handshake lol
            //  just track the connection rn
            handle_signaling_node(directory_ref, _socket, peer_addr).await;
        });
    }
}

async fn handle_signaling_node(
    _dir: PeerDirectory, 
    _socket: tokio::net::TcpStream, 
    _addr: SocketAddr
) {
    // handle the Register/Lookup loop
}