use tokio_util::codec::{Decoder, Encoder};
use bytes::BytesMut;
use crate::frame::Frame;
use crate::error::ProtocolError;
use crate::messages::SentinelMessage; 

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
        // 1. Decode the raw bytes into a Frame first
        match Frame::decode(src)? {
            Some(frame) => {
                // 2. Deserialize the payload into a SentinelMessage
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
        // 1. Serialize the message into bytes
        let payload = item.to_bytes();
        
        // 2. Wrap it in a Frame (Version 1, Flags 0 for now)
        let frame = Frame::new(1, 0, bytes::Bytes::from(payload))?;
        
        // 3. Encode the frame into the buffer
        frame.encode(dst)
    }
}