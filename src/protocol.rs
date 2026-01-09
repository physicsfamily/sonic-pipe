use crate::error::{Result, SonicPipeError};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_PAYLOAD_SIZE: usize = 1024;
pub const HEADER_SIZE: usize = 4;

#[derive(Debug, Clone)]
pub struct Packet {
    pub version: u8,
    pub payload_len: u16,
    pub flags: u8,
    pub payload: Vec<u8>,
    pub checksum: u32,
}

impl Packet {
    pub fn new(payload: Vec<u8>) -> Result<Self> {
        if payload.len() > MAX_PAYLOAD_SIZE {
            return Err(SonicPipeError::InvalidPacket(format!(
                "Payload too large: {} > {}",
                payload.len(),
                MAX_PAYLOAD_SIZE
            )));
        }

        let checksum = crc32fast::hash(&payload);

        Ok(Self {
            version: PROTOCOL_VERSION,
            payload_len: payload.len() as u16,
            flags: 0,
            payload,
            checksum,
        })
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(HEADER_SIZE + self.payload.len() + 4);

        data.push(self.version);
        data.write_u16::<BigEndian>(self.payload_len).unwrap();
        data.push(self.flags);
        data.extend_from_slice(&self.payload);
        data.write_u32::<BigEndian>(self.checksum).unwrap();

        data
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_SIZE + 4 {
            return Err(SonicPipeError::InvalidPacket("Data too short".into()));
        }

        let mut cursor = Cursor::new(data);

        let version = cursor.read_u8().map_err(|e| SonicPipeError::Decoding(e.to_string()))?;
        let payload_len = cursor.read_u16::<BigEndian>().map_err(|e| SonicPipeError::Decoding(e.to_string()))?;
        let flags = cursor.read_u8().map_err(|e| SonicPipeError::Decoding(e.to_string()))?;

        let payload_start = HEADER_SIZE;
        let payload_end = payload_start + payload_len as usize;

        if data.len() < payload_end + 4 {
            return Err(SonicPipeError::InvalidPacket("Incomplete packet".into()));
        }

        let payload = data[payload_start..payload_end].to_vec();

        let mut checksum_cursor = Cursor::new(&data[payload_end..]);
        let checksum = checksum_cursor.read_u32::<BigEndian>().map_err(|e| SonicPipeError::Decoding(e.to_string()))?;

        let computed_checksum = crc32fast::hash(&payload);
        if checksum != computed_checksum {
            return Err(SonicPipeError::ChecksumMismatch);
        }

        Ok(Self {
            version,
            payload_len,
            flags,
            payload,
            checksum,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_roundtrip() {
        let payload = b"Hello, Sonic-Pipe!".to_vec();
        let packet = Packet::new(payload.clone()).unwrap();
        let serialized = packet.serialize();
        let deserialized = Packet::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.version, PROTOCOL_VERSION);
        assert_eq!(deserialized.payload, payload);
    }
}
