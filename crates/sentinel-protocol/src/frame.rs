use bytes::{Bytes, BytesMut, Buf, BufMut};
use crate::error::ProtocolError;
use crc32fast::Hasher;

pub const MAGIC: [u8; 4] = *b"SNTL";
pub const MAGIC_LEN: usize = 4;
pub const VERSION_LEN: usize = 1;
pub const FLAGS_LEN: usize = 1;
pub const LENGTH_LEN: usize = 4;
pub const CRC_LEN: usize = 4;

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

    fn calculate_crc(version: u8, flags: u8, payload: &[u8]) -> u32 {
        let mut hasher = Hasher::new();
        hasher.update(&[version, flags]);
        hasher.update(payload);
        hasher.finalize()
    }

    pub fn decode(src: &mut BytesMut) -> Result<Option<Self>, ProtocolError> {
        if src.len() < HEADER_SIZE {
            return Ok(None);
        }

        if &src[0..MAGIC_LEN] != MAGIC {
            return Err(ProtocolError::InvalidMagic);
        }

        let payload_len_u32 = u32::from_be_bytes([
            src[LENGTH_OFFSET],
            src[LENGTH_OFFSET + 1],
            src[LENGTH_OFFSET + 2],
            src[LENGTH_OFFSET + 3],
        ]);
        
        let payload_len = usize::try_from(payload_len_u32)
            .map_err(|_| ProtocolError::FrameTooLarge)?;

        if payload_len > MAX_FRAME_SIZE {
            return Err(ProtocolError::FrameTooLarge);
        }
        // if payload_len == 0 {
        //     return Err(ProtocolError::ZeroLengthFrame);
        // }

        let total_size = HEADER_SIZE + payload_len + CRC_LEN;
        if src.len() < total_size {
            return Ok(None);
        }

        let version = src[VERSION_OFFSET];
        let flags = src[FLAGS_OFFSET];
        let payload_start = HEADER_SIZE;
        let payload_end = HEADER_SIZE + payload_len;

        let incoming_crc = u32::from_be_bytes([
            src[payload_end],
            src[payload_end + 1],
            src[payload_end + 2],
            src[payload_end + 3],
        ]);

        let computed_crc = Self::calculate_crc(version, flags, &src[payload_start..payload_end]);

        if incoming_crc != computed_crc {
            return Err(ProtocolError::IntegrityCheckFailed);
        }

        if version != SUPPORTED_VERSION {
            return Err(ProtocolError::UnsupportedVersion(version));
        }

        src.advance(HEADER_SIZE);
        let payload = src.split_to(payload_len).freeze();
        src.advance(CRC_LEN);

        Ok(Some(Frame { version, flags, payload }))
    }

    pub fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        let payload_len = self.payload.len();
        dst.reserve(HEADER_SIZE + payload_len + CRC_LEN);

        dst.put_slice(&MAGIC);
        dst.put_u8(self.version);
        dst.put_u8(self.flags);
        dst.put_u32(payload_len as u32);
        
        dst.extend_from_slice(&self.payload);

        let crc = Self::calculate_crc(self.version, self.flags, &self.payload);
        dst.put_u32(crc);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = Frame::new(SUPPORTED_VERSION, 0xAB, Bytes::from("sentinel")).unwrap();
        let mut buffer = BytesMut::new();
        original.encode(&mut buffer).unwrap();
        let decoded = Frame::decode(&mut buffer).unwrap().unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_integrity_failure() {
        let original = Frame::new(SUPPORTED_VERSION, 0x00, Bytes::from("data")).unwrap();
        let mut buffer = BytesMut::new();
        original.encode(&mut buffer).unwrap();
        let len = buffer.len();
        buffer[len - 1] ^= 0xFF; 
        assert!(matches!(Frame::decode(&mut buffer), Err(ProtocolError::IntegrityCheckFailed)));
    }
}