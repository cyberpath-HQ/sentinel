//! Compression trait and implementations

use async_trait::async_trait;

/// Compression algorithms available for WAL file rotation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionAlgorithm {
    /// Zstandard compression (high compression ratio, fast)
    Zstd,
    /// LZ4 compression (fast, moderate compression)
    Lz4,
    /// Brotli compression (high compression, slower)
    Brotli,
    /// DEFLATE compression (standard gzip)
    Deflate,
    /// GZIP compression (DEFLATE with header)
    Gzip,
}

/// Trait for compression implementations
#[async_trait]
pub trait CompressionTrait {
    /// Compress data
    async fn compress(&self, data: &[u8]) -> crate::Result<Vec<u8>>;
    /// Decompress data
    async fn decompress(&self, data: &[u8]) -> crate::Result<Vec<u8>>;
}

/// Zstandard compressor
pub struct ZstdCompressor;

#[async_trait]
impl CompressionTrait for ZstdCompressor {
    async fn compress(&self, data: &[u8]) -> crate::Result<Vec<u8>> {
        use async_compression::tokio::bufread::ZstdEncoder;
        use tokio::io::AsyncReadExt;

        let mut encoder = ZstdEncoder::new(std::io::Cursor::new(data));
        let mut compressed: Vec<u8> = Vec::new();
        encoder
            .read_to_end(&mut compressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("Zstd compression error: {}", e)))?;
        Ok(compressed)
    }

    async fn decompress(&self, data: &[u8]) -> crate::Result<Vec<u8>> {
        use async_compression::tokio::bufread::ZstdDecoder;
        use tokio::io::AsyncReadExt;

        let mut decoder = ZstdDecoder::new(std::io::Cursor::new(data));
        let mut decompressed: Vec<u8> = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("Zstd decompression error: {}", e)))?;
        Ok(decompressed)
    }
}

/// LZ4 compressor
pub struct Lz4Compressor;

#[async_trait]
impl CompressionTrait for Lz4Compressor {
    async fn compress(&self, data: &[u8]) -> crate::Result<Vec<u8>> {
        use async_compression::tokio::bufread::Lz4Encoder;
        use tokio::io::AsyncReadExt;

        let mut encoder = Lz4Encoder::new(std::io::Cursor::new(data));
        let mut compressed: Vec<u8> = Vec::new();
        encoder
            .read_to_end(&mut compressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("LZ4 compression error: {}", e)))?;
        Ok(compressed)
    }

    async fn decompress(&self, data: &[u8]) -> crate::Result<Vec<u8>> {
        use async_compression::tokio::bufread::Lz4Decoder;
        use tokio::io::AsyncReadExt;

        let mut decoder = Lz4Decoder::new(std::io::Cursor::new(data));
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("LZ4 decompression error: {}", e)))?;
        Ok(decompressed)
    }
}

/// Brotli compressor
pub struct BrotliCompressor;

#[async_trait]
impl CompressionTrait for BrotliCompressor {
    async fn compress(&self, data: &[u8]) -> crate::Result<Vec<u8>> {
        use async_compression::tokio::bufread::BrotliEncoder;
        use tokio::io::AsyncReadExt;

        let mut encoder = BrotliEncoder::new(std::io::Cursor::new(data));
        let mut compressed = Vec::new();
        encoder
            .read_to_end(&mut compressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("Brotli compression error: {}", e)))?;
        Ok(compressed)
    }

    async fn decompress(&self, data: &[u8]) -> crate::Result<Vec<u8>> {
        use async_compression::tokio::bufread::BrotliDecoder;
        use tokio::io::AsyncReadExt;

        let mut decoder = BrotliDecoder::new(std::io::Cursor::new(data));
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("Brotli decompression error: {}", e)))?;
        Ok(decompressed)
    }
}

/// DEFLATE compressor
pub struct DeflateCompressor;

#[async_trait]
impl CompressionTrait for DeflateCompressor {
    async fn compress(&self, data: &[u8]) -> crate::Result<Vec<u8>> {
        use async_compression::tokio::bufread::DeflateEncoder;
        use tokio::io::AsyncReadExt;

        let mut encoder = DeflateEncoder::new(std::io::Cursor::new(data));
        let mut compressed = Vec::new();
        encoder
            .read_to_end(&mut compressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("DEFLATE compression error: {}", e)))?;
        Ok(compressed)
    }

    async fn decompress(&self, data: &[u8]) -> crate::Result<Vec<u8>> {
        use async_compression::tokio::bufread::DeflateDecoder;
        use tokio::io::AsyncReadExt;

        let mut decoder = DeflateDecoder::new(std::io::Cursor::new(data));
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("DEFLATE decompression error: {}", e)))?;
        Ok(decompressed)
    }
}

/// GZIP compressor
pub struct GzipCompressor;

#[async_trait]
impl CompressionTrait for GzipCompressor {
    async fn compress(&self, data: &[u8]) -> crate::Result<Vec<u8>> {
        use async_compression::tokio::bufread::GzipEncoder;
        use tokio::io::AsyncReadExt;

        let mut encoder = GzipEncoder::new(std::io::Cursor::new(data));
        let mut compressed = Vec::new();
        encoder
            .read_to_end(&mut compressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("GZIP compression error: {}", e)))?;
        Ok(compressed)
    }

    async fn decompress(&self, data: &[u8]) -> crate::Result<Vec<u8>> {
        use async_compression::tokio::bufread::GzipDecoder;
        use tokio::io::AsyncReadExt;

        let mut decoder = GzipDecoder::new(std::io::Cursor::new(data));
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .await
            .map_err(|e| crate::WalError::Serialization(format!("GZIP decompression error: {}", e)))?;
        Ok(decompressed)
    }
}
