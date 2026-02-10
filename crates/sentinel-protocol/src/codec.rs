use tokio_util::codec::{Decoder, Encoder};
use bytes::BytesMut;
use crate::frame::Frame;
use crate::error::ProtocolError;
use crate::messages::SentinelMessage;

/// Safety limit: 10MB per message to prevent memory exhaustion attacks
const MAX_FRAME_SIZE: usize = 10 * 1024 * 1024; 

pub struct SentinelCodec;

impl SentinelCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for SentinelCodec {
    type Item = SentinelMessage;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // First, check if we even have enough bytes to read a length header (assuming Frame uses 4 bytes)
        // If your Frame::decode already handles length-prefixing, we add the safety check there.
        
        match Frame::decode(src)? {
            Some(frame) => {
                // Safety check: Don't process massive payloads
                if frame.payload().len() > MAX_FRAME_SIZE {
                    return Err(ProtocolError::SerializationError("Message exceeds MAX_FRAME_SIZE".into()));
                }

                let msg = SentinelMessage::from_bytes(frame.payload())
                    .map_err(|e| ProtocolError::SerializationError(e.to_string()))?;
                Ok(Some(msg))
            }
            None => Ok(None),
        }
    }
}

impl Encoder<SentinelMessage> for SentinelCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: SentinelMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload = item.to_bytes();
        
        // Wrap it in a Frame (Version 1, Flags 0)
        let frame = Frame::new(1, 0, bytes::Bytes::from(payload))?;
        
        frame.encode(dst)
    }
}