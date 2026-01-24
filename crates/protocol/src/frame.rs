use bytes::{Bytes, BytesMut, Buf, BufMut};
use crate::error::ProtocolError;

pub const MAGIC: [u8; 4] = *b"SNTL";
pub const MAGIC_LEN: usize = 4;
pub const VERSION_LEN: usize = 1;
pub const FLAGS_LEN: usize = 1;
pub const LENGTH_LEN: usize = 4;

pub const VERSION_OFFSET: usize = MAGIC_LEN;
pub const FLAGS_OFFSET: usize = VERSION_OFFSET + VERSION_LEN;
pub const LENGTH_OFFSET: usize = FLAGS_OFFSET + FLAGS_LEN;
pub const HEADER_SIZE: usize = MAGIC_LEN + VERSION_LEN + FLAGS_LEN + LENGTH_LEN;

pub const MAX_FRAME_SIZE: usize = 10 * 1024 * 1024;
pub const SUPPORTED_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq)] 
pub struct Frame {
    version: u8,
    flags: u8,
    payload: Bytes,
}

impl Frame {
    pub fn new(version: u8, flags: u8, payload: Bytes) -> Result<Self, ProtocolError> {
        if version != SUPPORTED_VERSION {
            return Err(ProtocolError::UnsupportedVersion(version));
        }
        if payload.len() > MAX_FRAME_SIZE {
            return Err(ProtocolError::FrameTooLarge);
        }
        Ok(Self { version, flags, payload })
    }

    pub fn version(&self) -> u8 { self.version }
    pub fn flags(&self) -> u8 { self.flags }
    pub fn payload(&self) -> &Bytes { &self.payload }

    pub fn decode(src: &mut BytesMut) -> Result<Option<Self>, ProtocolError> {
        if src.len() < HEADER_SIZE {
            return Ok(None);
        }

        if &src[0..MAGIC_LEN] != MAGIC {
            return Err(ProtocolError::InvalidMagic);
        }

        let version = src[VERSION_OFFSET];
        if version != SUPPORTED_VERSION {
            return Err(ProtocolError::UnsupportedVersion(version));
        }

        let len_bytes = &src[LENGTH_OFFSET..LENGTH_OFFSET + LENGTH_LEN];
        let payload_len_u32 = u32::from_be_bytes(len_bytes.try_into().unwrap());
        let payload_len = usize::try_from(payload_len_u32)
            .map_err(|_| ProtocolError::FrameTooLarge)?;

        if payload_len > MAX_FRAME_SIZE {
            return Err(ProtocolError::FrameTooLarge);
        }
        if payload_len == 0 {
            return Err(ProtocolError::ZeroLengthFrame);
        }

        if src.len() < HEADER_SIZE + payload_len {
            return Ok(None);
        }

        let flags = src[FLAGS_OFFSET];
        src.advance(HEADER_SIZE);

        let payload = src.split_to(payload_len).freeze();

        Ok(Some(Frame { version, flags, payload }))
    }

    pub fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.reserve(HEADER_SIZE + self.payload.len());

        dst.put_slice(&MAGIC);
        dst.put_u8(self.version);
        dst.put_u8(self.flags);
        dst.put_u32(self.payload.len() as u32);
        
        dst.put(self.payload.clone());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = Frame::new(
            SUPPORTED_VERSION,
            0xAB,
            Bytes::from("hello world")
        ).unwrap();

        let mut buffer = BytesMut::new();
        original.encode(&mut buffer).unwrap();

        let decoded = Frame::decode(&mut buffer).unwrap().unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_decode_invalid_magic() {
        let mut buffer = BytesMut::from(&b"BAD!"[..]);
        buffer.put_u8(SUPPORTED_VERSION);
        buffer.put_u8(0);
        buffer.put_u32(0);
        assert!(matches!(Frame::decode(&mut buffer), Err(ProtocolError::InvalidMagic)));
    }
}