use tokio::net::TcpListener;
use std::sync::Arc;
use dashmap::DashMap;
use std::net::SocketAddr;

//  hold active nodes
type PeerDirectory = Arc<DashMap<String, SocketAddr>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let addr = "0.0.0.0:8888"; // The public meeting point
    let listener = TcpListener::bind(addr).await?;
    let directory: PeerDirectory = Arc::new(DashMap::new());

    println!("🛰️ SENTINEL SIGNALER live on {}", addr);

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        let dir = Arc::clone(&directory);

        tokio::spawn(async move {
            println!(" Discovery: Node connected from {}", peer_addr);
            // implement the Handshake 
            // to verify the Node's Identity and add them to the directory.
        });
    }
}