use crate::SentinelTransport; 
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct RawTcpTransport {
    pub(crate) inner: TcpStream, 
}

impl RawTcpTransport {
    pub fn new(inner: TcpStream) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl SentinelTransport for RawTcpTransport {
    fn peer_addr(&self) -> Result<SocketAddr, std::io::Error> {
        self.inner.peer_addr()
    }

    fn is_secure(&self) -> bool {
        false
    }
}

impl AsyncRead for RawTcpTransport {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for RawTcpTransport {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}