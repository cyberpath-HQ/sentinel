//! GZIP compression implementation
//!
//! GZIP is the most widely compatible compression format. It provides good compression
//! ratios with reasonable performance and is supported by virtually all systems and tools.
//! Use GZIP when maximum compatibility is required, such as for interchange with other
//! systems or when working with legacy tools that don't support newer algorithms.

use async_trait::async_trait;

use crate::{compression::CompressionTrait, Result};

/// GZIP compressor
pub struct GzipCompressor;

#[async_trait]
impl CompressionTrait for GzipCompressor {
    async fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use async_compression::tokio::bufread::GzipEncoder;
        use tokio::io::AsyncReadExt as _;

        let mut encoder = GzipEncoder::new(std::io::Cursor::new(data));
        let mut compressed: Vec<u8> = Vec::new();
        encoder
            .read_to_end(&mut compressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("GZIP compression error: {}", e)))?;
        Ok(compressed)
    }

    async fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use async_compression::tokio::bufread::GzipDecoder;
        use tokio::io::AsyncReadExt as _;

        let mut decoder = GzipDecoder::new(std::io::Cursor::new(data));
        let mut decompressed: Vec<u8> = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("GZIP decompression error: {}", e)))?;
        Ok(decompressed)
    }
}
