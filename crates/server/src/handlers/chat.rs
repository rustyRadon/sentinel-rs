use async_trait::async_trait;
use sentinel_protocol::commands::CommandHandler;
use sentinel_protocol::frame::Frame;
use sentinel_protocol::error::ProtocolError;
use bytes::Bytes;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use std::io::Write;

pub struct ChatHandler;

#[async_trait]
impl CommandHandler for ChatHandler {
    async fn handle(&self, frame: Frame) -> Result<Option<Frame>, ProtocolError> {
        let incoming_msg = String::from_utf8_lossy(frame.payload());
        
        println!("user 1> {}", incoming_msg);

        if incoming_msg == "exit" {
            println!("[SYSTEM]: Remote user ended session.");
            return Ok(Some(Frame::new(1, 0x05, Bytes::from("Session Closed"))?));
        }

        print!("user 2> ");
        std::io::stdout().flush().map_err(|_| ProtocolError::ZeroLengthFrame)?;

        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin).lines();

        if let Ok(Some(line)) = reader.next_line().await {
            let response = Frame::new(
                1,
                0x05,
                Bytes::from(line)
            )?;
            return Ok(Some(response));
        }

        Ok(None)
    }
}