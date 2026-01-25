use async_trait::async_trait;
use sentinel_protocol::commands::CommandHandler;
use sentinel_protocol::frame::Frame;
use sentinel_protocol::error::ProtocolError;
use bytes::Bytes;

pub struct SysInfoHandler;

#[async_trait]
impl CommandHandler for SysInfoHandler {
    async fn handle(&self, _frame: Frame) -> Result<Option<Frame>, ProtocolError> {
        let stats = format!("Sentinel-v1.0|Uptime:{}s", 3600); 
        
        let response = Frame::new(
            1, 
            0x00, 
            Bytes::from(stats)
        )?;
        
        Ok(Some(response))
    }
}