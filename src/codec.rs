use crate::error::{Result, SonicPipeError};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use reed_solomon_erasure::galois_8::ReedSolomon;

pub const ECC_DATA_SHARDS: usize = 8;
pub const ECC_PARITY_SHARDS: usize = 4;

pub fn compress(data: &[u8]) -> Vec<u8> {
    compress_prepend_size(data)
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    decompress_size_prepended(data)
        .map_err(|e| SonicPipeError::Compression(e.to_string()))
}

pub struct ReedSolomonCodec {
    rs: ReedSolomon,
    data_shards: usize,
    parity_shards: usize,
}

impl ReedSolomonCodec {
    pub fn new() -> Result<Self> {
        let rs = ReedSolomon::new(ECC_DATA_SHARDS, ECC_PARITY_SHARDS)
            .map_err(|e| SonicPipeError::ErrorCorrection(e.to_string()))?;

        Ok(Self {
            rs,
            data_shards: ECC_DATA_SHARDS,
            parity_shards: ECC_PARITY_SHARDS,
        })
    }

    pub fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        let shard_size = (data.len() + self.data_shards - 1) / self.data_shards;
        let total_shards = self.data_shards + self.parity_shards;

        let mut shards: Vec<Vec<u8>> = Vec::with_capacity(total_shards);

        for i in 0..self.data_shards {
            let start = i * shard_size;
            let end = std::cmp::min(start + shard_size, data.len());

            let mut shard = vec![0u8; shard_size];
            if start < data.len() {
                let copy_len = end - start;
                shard[..copy_len].copy_from_slice(&data[start..end]);
            }
            shards.push(shard);
        }

        for _ in 0..self.parity_shards {
            shards.push(vec![0u8; shard_size]);
        }

        self.rs
            .encode(&mut shards)
            .map_err(|e| SonicPipeError::ErrorCorrection(e.to_string()))?;

        let mut result = Vec::with_capacity(4 + data.len() + total_shards * shard_size);
        result.extend_from_slice(&(data.len() as u32).to_be_bytes());
        result.extend_from_slice(&(shard_size as u32).to_be_bytes());

        for shard in shards {
            result.extend_from_slice(&shard);
        }

        Ok(result)
    }

    pub fn decode(&self, encoded: &[u8]) -> Result<Vec<u8>> {
        if encoded.len() < 8 {
            return Err(SonicPipeError::ErrorCorrection("Data too short".into()));
        }

        let original_len = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]) as usize;
        let shard_size = u32::from_be_bytes([encoded[4], encoded[5], encoded[6], encoded[7]]) as usize;

        let total_shards = self.data_shards + self.parity_shards;
        let expected_len = 8 + total_shards * shard_size;

        if encoded.len() < expected_len {
            return Err(SonicPipeError::ErrorCorrection("Incomplete data".into()));
        }

        let mut shards: Vec<Option<Vec<u8>>> = Vec::with_capacity(total_shards);
        for i in 0..total_shards {
            let start = 8 + i * shard_size;
            let end = start + shard_size;
            shards.push(Some(encoded[start..end].to_vec()));
        }

        self.rs
            .reconstruct(&mut shards)
            .map_err(|e| SonicPipeError::ErrorCorrection(e.to_string()))?;

        let mut result = Vec::with_capacity(original_len);
        for shard in shards.iter().take(self.data_shards) {
            if let Some(data) = shard {
                result.extend_from_slice(data);
            }
        }

        result.truncate(original_len);
        Ok(result)
    }
}

impl Default for ReedSolomonCodec {
    fn default() -> Self {
        Self::new().expect("Failed to create Reed-Solomon codec")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_roundtrip() {
        let data = b"Hello, Sonic-Pipe! This is a test message.";
        let compressed = compress(data);
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_reed_solomon_roundtrip() {
        let codec = ReedSolomonCodec::new().unwrap();
        let data = b"Test data for Reed-Solomon encoding";
        let encoded = codec.encode(data).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }
}
