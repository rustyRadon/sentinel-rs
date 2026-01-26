use std::marker::PhantomData;
use async_trait::async_trait;
use sentinel_protocol::Frame;
use anyhow::Result;

#[async_trait]
pub trait SentinelTransport: Send + Sync {
    async fn send_frame(&mut self, frame: Frame) -> Result<()>;
    async fn next_frame(&mut self) -> Result<Option<Frame>>;
}

pub struct Unauthenticated;
pub struct Authenticated { pub user_id: String }

pub struct Connection<T: SentinelTransport, S> {
    pub transport: T,
    pub state_data: S,
    _state: PhantomData<S>,
}

impl<T: SentinelTransport> Connection<T, Unauthenticated> {
    pub fn new(transport: T) -> Self {
        Self { 
            transport, 
            state_data: Unauthenticated,
            _state: PhantomData 
        }
    }

    pub async fn send_frame(&mut self, frame: Frame) -> Result<()> {
        self.transport.send_frame(frame).await
    }

    pub async fn next_frame(&mut self) -> Result<Option<Frame>> {
        self.transport.next_frame().await
    }

    pub fn into_authenticated(self, user_id: String) -> Connection<T, Authenticated> {
        Connection {
            transport: self.transport,
            state_data: Authenticated { user_id },
            _state: PhantomData,
        }
    }
}

impl<T: SentinelTransport> Connection<T, Authenticated> {
    pub fn user_id(&self) -> &str {
        &self.state_data.user_id
    }

    pub async fn send_frame(&mut self, frame: Frame) -> Result<()> {
        self.transport.send_frame(frame).await
    }

    pub async fn next_frame(&mut self) -> Result<Option<Frame>> {
        self.transport.next_frame().await
    }
}