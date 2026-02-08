use socket2::{Socket, Domain, Type, Protocol, SockAddr};
use std::net::SocketAddr;
use anyhow::Result;

pub struct FighterSocket;

impl FighterSocket {
    /// creates a raw TCP socket with SO_REUSEADDR and SO_REUSEPORT.
    /// allows mee to "hijack" our own listening port for outbound dials.
    pub fn create_war_ready(local_addr: SocketAddr) -> Result<Socket> {
        let domain = if local_addr.is_ipv4() { Domain::IPV4 } else { Domain::IPV6 };
        
        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;

        // Enable address reuse (fixes "Address already in use" errors)
        socket.set_reuse_address(true)?;
        
        // Enable port reuse (Crucial for the simultaneous punch)
        #[cfg(all(unix, not(target_os = "solaris"), not(target_os = "illumos")))]
        socket.set_reuse_port(true)?;

        // Bind it to our local address/port before connecting
        socket.bind(&SockAddr::from(local_addr))?;
        
        // Set to non-blocking so Tokio can take over later
        socket.set_nonblocking(true)?;

        Ok(socket)
    }
}