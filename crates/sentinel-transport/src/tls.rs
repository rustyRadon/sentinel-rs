use crate::SentinelTransport;
use async_trait::async_trait;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::TlsStream;
use tokio::net::TcpStream;

pub struct TlsTransport<S = TcpStream> {
    pub(crate) inner: TlsStream<S>,
}

impl<S> TlsTransport<S> {
    pub fn new(inner: TlsStream<S>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<S> SentinelTransport for TlsTransport<S> 
where 
    S: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
{
    fn peer_addr(&self) -> Result<SocketAddr, std::io::Error> {
        let (raw_stream, _) = self.inner.get_ref();
        unsafe {
            let tcp_ptr = raw_stream as *const S as *const TcpStream;
            (*tcp_ptr).peer_addr()
        }
    }

    fn is_secure(&self) -> bool {
        true
    }
}

impl<S> AsyncRead for TlsTransport<S> 
where S: AsyncRead + AsyncWrite + Unpin 
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<S> AsyncWrite for TlsTransport<S> 
where S: AsyncRead + AsyncWrite + Unpin 
{
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