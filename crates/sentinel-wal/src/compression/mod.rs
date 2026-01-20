//! Compression algorithms for WAL file rotation

pub mod brotli;
pub mod deflate;
pub mod gzip;
pub mod lz4;
pub mod zstd;

pub use zstd::ZstdCompressor;
pub use lz4::Lz4Compressor;
pub use brotli::BrotliCompressor;
pub use deflate::DeflateCompressor;
pub use gzip::GzipCompressor;

/// Compression algorithms available for WAL file rotation
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompressionAlgorithm {
    /// Zstandard compression: Best overall choice for WAL files.
    /// Provides excellent compression ratio (better than gzip) with fast compression/decompression
    /// speeds. Ideal for production environments where storage space is important but
    /// performance is critical. Recommended for most use cases.
    Zstd,
    /// LZ4 compression: Fastest compression and decompression.
    /// Lower compression ratio than Zstd but very fast.
    /// Suitable for high-throughput environments where speed is more important than compression
    /// ratio. Good for real-time systems with limited CPU resources.
    Lz4,
    /// Brotli compression: Highest compression ratio.
    /// Slower than Zstd but achieves better compression.
    /// Best for archival or low-frequency rotation scenarios where maximum compression is desired.
    /// Use when storage space is at a premium and compression time is not critical.
    Brotli,
    /// DEFLATE compression: Standard compression algorithm.
    /// Balanced performance with good compatibility.
    /// Suitable for environments requiring standard compression formats.
    /// Good default for general-purpose use.
    Deflate,
    /// GZIP compression: DEFLATE with gzip header.
    /// Widely compatible and standard for many systems.
    /// Slightly slower than DEFLATE due to header overhead.
    /// Use when compatibility with existing gzip tools is required.
    Gzip,
}

impl std::str::FromStr for CompressionAlgorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "zstd" => Ok(Self::Zstd),
            "lz4" => Ok(Self::Lz4),
            "brotli" => Ok(Self::Brotli),
            "deflate" => Ok(Self::Deflate),
            "gzip" => Ok(Self::Gzip),
            _ => Err(format!("Invalid compression algorithm: {}", s)),
        }
    }
}

impl std::fmt::Display for CompressionAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Zstd => write!(f, "zstd"),
            Self::Lz4 => write!(f, "lz4"),
            Self::Brotli => write!(f, "brotli"),
            Self::Deflate => write!(f, "deflate"),
            Self::Gzip => write!(f, "gzip"),
        }
    }
}

/// Trait for compression implementations
#[async_trait::async_trait]
pub trait CompressionTrait {
    /// Compress data
    async fn compress(&self, data: &[u8]) -> crate::Result<Vec<u8>>;
    /// Decompress data
    async fn decompress(&self, data: &[u8]) -> crate::Result<Vec<u8>>;
}

/// Get a compressor instance for the given algorithm
pub fn get_compressor(algorithm: CompressionAlgorithm) -> Box<dyn CompressionTrait + Send + Sync> {
    match algorithm {
        CompressionAlgorithm::Zstd => Box::new(ZstdCompressor),
        CompressionAlgorithm::Lz4 => Box::new(Lz4Compressor),
        CompressionAlgorithm::Brotli => Box::new(BrotliCompressor),
        CompressionAlgorithm::Deflate => Box::new(DeflateCompressor),
        CompressionAlgorithm::Gzip => Box::new(GzipCompressor),
    }
}
