use async_trait::async_trait;
use sentinel_protocol::commands::CommandHandler;
use sentinel_protocol::frame::Frame;
use sentinel_protocol::error::ProtocolError;
use bytes::Bytes;

pub struct ChatHandler;

#[async_trait]
impl CommandHandler for ChatHandler {
    async fn handle(&self, frame: Frame) -> Result<Option<Frame>, ProtocolError> {
        let msg = String::from_utf8_lossy(frame.payload());
        
        println!("[REMOTE_CHAT]: {}", msg);

        let response = Frame::new(
            1, 
            0x05, 
            Bytes::from("Message received")
        )?;
        
        Ok(Some(response))
    }
}