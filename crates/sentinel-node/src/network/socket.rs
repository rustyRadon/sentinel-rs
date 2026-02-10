use socket2::{Socket, Domain, Type, Protocol, SockAddr};
use std::net::{SocketAddr, ToSocketAddrs};
use anyhow::{Result, anyhow};
use stunclient::StunClient; 

pub struct FighterSocket;

impl FighterSocket {
    /// discover our public identity before creating the TCP socket.
    pub async fn discover_public_ip(local_port: u16) -> Result<SocketAddr> {
        let local_udp_addr: SocketAddr = format!("0.0.0.0:{}", local_port).parse()?;
        let udp_socket = tokio::net::UdpSocket::bind(local_udp_addr).await?;

        let stun_server = "stun.l.google.com:19302"
            .to_socket_addrs()?
            .find(|x| x.is_ipv4())
            .ok_or_else(|| anyhow!("Failed to resolve STUN server"))?;

        let client = StunClient::new(stun_server);
        
        // 'punches' a hole and returns how the internet sees ussss
        let public_addr = client.query_external_address_async(&udp_socket).await
            .map_err(|e| anyhow!("STUN query failed: {}", e))?;

        Ok(public_addr)
    }

    /// war ready lmao
    pub fn create_war_ready(local_addr: SocketAddr) -> Result<Socket> {
        let domain = if local_addr.is_ipv6() { Domain::IPV6 } else { Domain::IPV4 };
        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;

        socket.set_reuse_address(true)?;
        
        #[cfg(all(unix, not(target_os = "solaris"), not(target_os = "illumos")))]
        socket.set_reuse_port(true)?;

        socket.bind(&SockAddr::from(local_addr))?;
        socket.set_nonblocking(true)?;

        Ok(socket)
    }
}