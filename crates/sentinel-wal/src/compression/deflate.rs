//! DEFLATE compression implementation
//!
//! DEFLATE provides good compression ratios with reasonable speed. It's widely compatible
//! and a good middle-ground choice when you need better compression than LZ4 but don't
//! want the complexity of Brotli. Use DEFLATE for general-purpose compression where
//! compatibility with other tools is important.

use async_trait::async_trait;

use crate::{compression::CompressionTrait, Result};

/// DEFLATE compressor
pub struct DeflateCompressor;

#[async_trait]
impl CompressionTrait for DeflateCompressor {
    async fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use async_compression::tokio::bufread::DeflateEncoder;
        use tokio::io::AsyncReadExt as _;

        let mut encoder = DeflateEncoder::new(std::io::Cursor::new(data));
        let mut compressed: Vec<u8> = Vec::new();
        encoder
            .read_to_end(&mut compressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("DEFLATE compression error: {}", e)))?;
        Ok(compressed)
    }

    async fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use async_compression::tokio::bufread::DeflateDecoder;
        use tokio::io::AsyncReadExt as _;

        let mut decoder = DeflateDecoder::new(std::io::Cursor::new(data));
        let mut decompressed: Vec<u8> = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("DEFLATE decompression error: {}", e)))?;
        Ok(decompressed)
    }
}
