//! Zstandard compression implementation
//!
//! Zstandard (Zstd) provides the best balance of compression ratio and speed for WAL files.
//! It typically achieves 2-3x better compression than gzip with similar or faster decompression.
//! Use Zstd for most production scenarios where both performance and storage efficiency matter.

use async_trait::async_trait;

use crate::{compression::CompressionTrait, Result};

/// Zstandard compressor
pub struct ZstdCompressor;

#[async_trait]
impl CompressionTrait for ZstdCompressor {
    async fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use async_compression::tokio::bufread::ZstdEncoder;
        use tokio::io::AsyncReadExt as _;

        let mut encoder = ZstdEncoder::new(std::io::Cursor::new(data));
        let mut compressed: Vec<u8> = Vec::new();
        encoder
            .read_to_end(&mut compressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("Zstd compression error: {}", e)))?;
        Ok(compressed)
    }

    async fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use async_compression::tokio::bufread::ZstdDecoder;
        use tokio::io::AsyncReadExt as _;

        let mut decoder = ZstdDecoder::new(std::io::Cursor::new(data));
        let mut decompressed: Vec<u8> = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("Zstd decompression error: {}", e)))?;
        Ok(decompressed)
    }
}
