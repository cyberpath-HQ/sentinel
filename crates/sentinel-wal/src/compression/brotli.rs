//! Brotli compression implementation
//!
//! Brotli provides excellent compression ratios (often better than gzip) at the cost of
//! slower compression speeds. Decompression is reasonably fast. Use Brotli when maximum
//! storage efficiency is needed and compression time is not critical, such as for
//! archival or long-term storage scenarios.

use async_trait::async_trait;

use crate::{compression::CompressionTrait, Result};

/// Brotli compressor
pub struct BrotliCompressor;

#[async_trait]
impl CompressionTrait for BrotliCompressor {
    async fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use async_compression::tokio::bufread::BrotliEncoder;
        use tokio::io::AsyncReadExt as _;

        let mut encoder = BrotliEncoder::new(std::io::Cursor::new(data));
        let mut compressed: Vec<u8> = Vec::new();
        encoder
            .read_to_end(&mut compressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("Brotli compression error: {}", e)))?;
        Ok(compressed)
    }

    async fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use async_compression::tokio::bufread::BrotliDecoder;
        use tokio::io::AsyncReadExt as _;

        let mut decoder = BrotliDecoder::new(std::io::Cursor::new(data));
        let mut decompressed: Vec<u8> = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("Brotli decompression error: {}", e)))?;
        Ok(decompressed)
    }
}
