use crate::SentinelTransport;
use async_trait::async_trait;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::server::TlsStream as ServerTlsStream;
use tokio::net::TcpStream;

pub struct TlsTransport {
    pub(crate) inner: ServerTlsStream<TcpStream>,
}

impl TlsTransport {
    pub fn new(inner: ServerTlsStream<TcpStream>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl SentinelTransport for TlsTransport {
    fn peer_addr(&self) -> Result<SocketAddr, std::io::Error> {
        let (tcp, _) = self.inner.get_ref();
        tcp.peer_addr()
    }

    fn is_secure(&self) -> bool {
        true
    }
}

impl AsyncRead for TlsTransport {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for TlsTransport {
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