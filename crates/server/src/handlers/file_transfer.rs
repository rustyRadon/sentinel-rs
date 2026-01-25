use async_trait::async_trait;
use sentinel_protocol::commands::CommandHandler;
use sentinel_protocol::frame::Frame;
use sentinel_protocol::error::ProtocolError;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use bytes::Bytes;

pub struct FileUploadHandler;

#[async_trait]
impl CommandHandler for FileUploadHandler {
    async fn handle(&self, frame: Frame) -> Result<Option<Frame>, ProtocolError> {
        let payload = frame.payload();
        
        if payload.len() < 3 {
            return Err(ProtocolError::FrameTooLarge);
        }

        let name_len = payload[1] as usize;
        let filename = String::from_utf8_lossy(&payload[2..2 + name_len]).to_string();
        let file_data = &payload[2 + name_len..];

        let safe_name = filename.replace("..", "").replace("/", "");
        let mut file = File::create(format!("./uploads/{}", safe_name)).await
            .map_err(|_| ProtocolError::ZeroLengthFrame)?; 

        file.write_all(file_data).await.map_err(|_| ProtocolError::FrameTooLarge)?;

        let response = Frame::new(
            1, 
            0x00, 
            Bytes::from(format!("Successfully saved {}", safe_name))
        )?;
        
        Ok(Some(response))
    }
}