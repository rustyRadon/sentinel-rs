use tokio_util::codec::{Decoder, Encoder};
use bytes::BytesMut;
use crate::frame::Frame;
use crate::error::ProtocolError;

pub struct SentinelCodec;

impl SentinelCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for SentinelCodec {
    type Item = Frame;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Frame::decode(src)
    }
}

impl Encoder<Frame> for SentinelCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.encode(dst)
    }
}