//! LZ4 compression implementation
//!
//! LZ4 offers the fastest compression and decompression speeds among the available algorithms.
//! It provides moderate compression ratios (typically 1.5-2x) but excels in scenarios where
//! speed is critical and storage space is less of a concern. Ideal for high-throughput
//! environments or when CPU resources are limited.

use async_trait::async_trait;

use crate::{compression::CompressionTrait, Result};

/// LZ4 compressor
pub struct Lz4Compressor;

#[async_trait]
impl CompressionTrait for Lz4Compressor {
    async fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use async_compression::tokio::bufread::Lz4Encoder;
        use tokio::io::AsyncReadExt as _;

        let mut encoder = Lz4Encoder::new(std::io::Cursor::new(data));
        let mut compressed: Vec<u8> = Vec::new();
        encoder
            .read_to_end(&mut compressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("LZ4 compression error: {}", e)))?;
        Ok(compressed)
    }

    async fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use async_compression::tokio::bufread::Lz4Decoder;
        use tokio::io::AsyncReadExt as _;

        let mut decoder = Lz4Decoder::new(std::io::Cursor::new(data));
        let mut decompressed: Vec<u8> = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("LZ4 decompression error: {}", e)))?;
        Ok(decompressed)
    }
}
