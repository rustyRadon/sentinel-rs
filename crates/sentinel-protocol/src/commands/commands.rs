use async_trait::async_trait;
use crate::frame::Frame;
use crate::error::ProtocolError;

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn handle(&self, frame: Frame) -> Result<Option<Frame>, ProtocolError>;
}