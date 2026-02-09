use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;
use dashmap::DashMap;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use std::sync::Arc;
use futures::{StreamExt, SinkExt, future::{BoxFuture, FutureExt}}; 
use tokio_util::codec::Framed;
use lru::LruCache;
use tokio::net::TcpStream as TokioTcpStream;

use crate::network::FighterSocket;
use std::net::{SocketAddr, ToSocketAddrs};

use sentinel_crypto::NodeIdentity;
use sentinel_protocol::{
    SentinelCodec, 
    messages::{SentinelMessage, MessageContent, PeerInfo, SignalingMessage}
};
use sentinel_transport::{SentinelAcceptor, SentinelConnector};
use mdns_sd::ServiceDaemon;

pub struct PeerState {
    pub tx: mpsc::UnboundedSender<SentinelMessage>,
    pub node_id: String,
    pub node_name: String,
    pub public_key: Option<Vec<u8>>,
}

pub struct SentinelNode {
    pub identity: NodeIdentity,
    pub acceptor: SentinelAcceptor,
    pub db: sled::Db,
    pub mdns: ServiceDaemon,
    pub peers: DashMap<String, PeerState>,
    pub seen_messages: Mutex<LruCache<Uuid, ()>>,
    pub signaler_tx: mpsc::UnboundedSender<SentinelMessage>,
}

impl SentinelNode {
    /// Returns the Node instance and the Receiver for the signaling task
    pub async fn new(data_dir: PathBuf) -> Result<(Self, mpsc::UnboundedReceiver<SentinelMessage>)> {
        if !data_dir.exists() { std::fs::create_dir_all(&data_dir)?; }
        let identity = NodeIdentity::load_or_generate(data_dir.join("identity.key"))?;
        let db = sled::open(data_dir.join("storage.db"))?;
        
        let cert_path = if data_dir.join("node.crt").exists() { data_dir.join("node.crt") } else { PathBuf::from("certs/server.crt") };
        let key_path = if data_dir.join("node.key").exists() { data_dir.join("node.key") } else { PathBuf::from("certs/server.key") };
        
        let acceptor = SentinelAcceptor::new(&cert_path, &key_path, Duration::from_secs(10))?;
        let mdns = ServiceDaemon::new().context("mDNS failed")?;
        let seen_messages = Mutex::new(LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()));
        
        let (signaler_tx, signaler_rx) = mpsc::unbounded_channel();

        Ok((Self { 
            identity, acceptor, db, mdns, peers: DashMap::new(), seen_messages, signaler_tx 
        }, signaler_rx))
    }

    pub fn sign_and_send(&self, tx: &mpsc::UnboundedSender<SentinelMessage>, mut msg: SentinelMessage) {
        msg.public_key = self.identity.public_key_bytes();
        msg.signature = self.identity.sign(&msg.sig_hash());
        let _ = tx.send(msg);
    }

    pub async fn start_signaler_client(
        self: Arc<Self>, 
        signaler_addr: String, 
        mut signaler_outbound: mpsc::UnboundedReceiver<SentinelMessage>
    ) {
        loop {
            println!("Connecting to Signaler at {}...", signaler_addr);
            match tokio::net::TcpStream::connect(&signaler_addr).await {
                Ok(stream) => {
                    let mut framed = Framed::new(stream, SentinelCodec::new());
                    let my_id = self.identity.node_id();
                    
                    let reg_msg = SentinelMessage::new_signal(
                        my_id.clone(),
                        SignalingMessage::Register {
                            node_id: my_id.clone(),
                            public_key: self.identity.public_key_bytes(),
                            signature: vec![], 
                        },
                    );

                    if framed.send(reg_msg).await.is_ok() {
                        println!("Registered with Signaler as {}", my_id);
                        
                        let (mut sink, mut stream) = framed.split();

                        loop {
                            tokio::select! {
                                // Handle outbound requests from our CLI (like LookupRequest)
                                Some(outbound_msg) = signaler_outbound.recv() => {
                                    if sink.send(outbound_msg).await.is_err() { break; }
                                }
                                // Handle inbound responses from the Signaler
                                Some(result) = stream.next() => {
                                    match result {
                                        Ok(msg) => {
                                            if let MessageContent::Signal(signal) = msg.content {
                                                match signal {
                                                    SignalingMessage::PeerResponse { peer_id, public_addr } => {
                                                        println!("Signaler found peer {} at {}. Dialing...", peer_id, public_addr);
                                                        let node_clone = Arc::clone(&self);
                                                        tokio::spawn(async move {
                                                            let _ = node_clone.dial_peer(public_addr.to_string()).await;
                                                        });
                                                    }
                                                    SignalingMessage::PunchCommand { target_addr, .. } => {
                                                        println!("Incoming Punch Command for {}. Preparation starting...", target_addr);
                                                    }
                                                    SignalingMessage::Error(e) => eprintln!("Signaler Error: {}", e),
                                                    _ => {}
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("Signaler stream error: {}", e);
                                            break;
                                        }
                                    }
                                }
                                else => break,
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Signaler offline: {}. Retrying in 5s...", e),
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    pub fn handle_incoming_message(self: Arc<Self>, msg: SentinelMessage, addr: String) -> BoxFuture<'static, Result<()>> {
        let node = self.clone();
        async move {
            {
                let mut seen = node.seen_messages.lock().await;
                if seen.contains(&msg.id) { return Ok(()); }
                seen.put(msg.id, ());
            }

            if !msg.signature.is_empty() && !msg.public_key.is_empty() {
                if !NodeIdentity::verify(&msg.sig_hash(), &msg.signature, &msg.public_key) {
                    return Ok(());
                }
            }

            let sender_id = msg.sender.clone();
            let content = msg.content.clone();

            match content {
                MessageContent::Handshake { public_key, node_name } => {
                    println!("Peer Verified: {} as {}", node_name, sender_id);
                    if let Some(mut peer) = node.peers.get_mut(&addr) {
                        peer.node_id = sender_id;
                        peer.node_name = node_name;
                        peer.public_key = Some(public_key);
                    }
                }
                MessageContent::Chat(text) => {
                    println!("[{}] (Chat): {}", sender_id, text);
                    let _ = node.persist_message(&msg);
                    for entry in node.peers.iter() {
                        if entry.key() != &addr { 
                            let _ = entry.value().tx.send(msg.clone()); 
                        }
                    }
                }
                MessageContent::PeerDiscovery(new_peers) => {
                    for peer in new_peers {
                        if peer.node_id != node.identity.node_id() && !node.peers.contains_key(&peer.address.to_string()) {
                            let node_clone = Arc::clone(&node);
                            tokio::spawn(async move { let _ = node_clone.dial_peer(peer.address.to_string()).await; });
                        }
                    }
                }
                MessageContent::Ping => { 
                    let _ = node.send_to_peer(&addr, MessageContent::Pong).await; 
                }
                _ => {}
            }
            Ok(())
        }.boxed() 
    }

    pub async fn dial_peer(self: Arc<Self>, addr: String) -> Result<()> {
       
        let target_addr: SocketAddr = addr
            .to_socket_addrs()?
            .next()
            .context("Failed to resolve target address")?;

        let local_bind = SocketAddr::from(([0, 0, 0, 0], 0));
        let fighter = FighterSocket::create_war_ready(local_bind)?;

        // initiate connection (The Punch) typeshiiii
        println!("Fighter socket punching through to {}...", target_addr);
        
        // socket2's connect directly 
        match fighter.connect(&target_addr.into()) {
            Ok(_) => {}
            Err(e) => {
                // since it's non-blocking, EINPROGRESS is expected
                if e.raw_os_error() != Some(115) && e.kind() != std::io::ErrorKind::WouldBlock {
                    return Err(anyhow::anyhow!("Fighter punch failed: {}", e));
                }
            }
        }

        // over to Tokio hehehehe
        let std_stream: std::net::TcpStream = fighter.into();
        let tokio_stream = TokioTcpStream::from_std(std_stream)
            .context("Failed to hand over fighter socket to Tokio")?;

        // gotta wait for the socket to be ready
        tokio_stream.writable().await?;
        if let Some(e) = tokio_stream.take_error()? {
            return Err(anyhow::anyhow!("Socket error after punch: {}", e));
        }

        //  to TLS... saftyy
        let connector = SentinelConnector::new();
        let tls = connector.connect("sentinel-node.local", tokio_stream).await?;

        // wrap in Framed Codec ( makes it a "Stream")
        let (mut sink, mut stream) = Framed::new(tls, SentinelCodec::new()).split();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // track peer state
        self.peers.insert(addr.clone(), PeerState { 
            tx: tx.clone(), 
            node_id: "pending".into(), 
            node_name: "new-peer".into(), 
            public_key: None 
        });

        // send Handshake
        let hs = SentinelMessage::new(self.identity.node_id(), MessageContent::Handshake {
            public_key: self.identity.public_key_bytes(),
            node_name: "Sentinel-Node".into(),
        });
        self.sign_and_send(&tx, hs);

        // tasks for IO
        let addr_io = addr.clone();
        
        // Task A: Outbound (Forward messages from channel to socket)
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if sink.send(msg).await.is_err() { break; }
            }
        });

        // Task B: Inbound (Process messages from socket)
        let node_inner = Arc::clone(&self);
        tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                let _ = node_inner.clone().handle_incoming_message(msg, addr_io.clone()).await;
            }
            node_inner.peers.remove(&addr_io);
        });

        Ok(())
    }

    pub async fn start_gossip_service(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let peer_list: Vec<PeerInfo> = self.peers.iter().filter_map(|e| {
                e.key().parse().ok().map(|addr| PeerInfo {
                    node_id: e.value().node_id.clone(),
                    address: addr,
                    node_name: e.value().node_name.clone(),
                    last_seen: 0,
                })
            }).collect();
            
            if !peer_list.is_empty() {
                let msg = SentinelMessage::new(self.identity.node_id(), MessageContent::PeerDiscovery(peer_list));
                for entry in self.peers.iter() { 
                    self.sign_and_send(&entry.value().tx, msg.clone()); 
                }
            }
        }
    }
    

    pub async fn send_to_peer(&self, addr: &str, content: MessageContent) -> Result<()> {
        if let Some(peer) = self.peers.get(addr) {
            self.sign_and_send(&peer.tx, SentinelMessage::new(self.identity.node_id(), content));
        }
        Ok(())
    }

    pub fn persist_message(&self, msg: &SentinelMessage) -> Result<()> {
        let tree = self.db.open_tree("messages")?;
        tree.insert(format!("{}:{}", msg.timestamp, msg.sender), msg.to_bytes())?;
        Ok(())
    }

    pub fn print_history(&self) -> Result<()> {
        if let Ok(tree) = self.db.open_tree("messages") {
            for item in tree.iter().values().rev().take(10).flatten() {
                if let Ok(msg) = SentinelMessage::from_bytes(&item) {
                    println!("[{}] {:?}", msg.sender, msg.content);
                }
            }
        }
        Ok(())
    }
}