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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_compression_algorithm_from_str_valid() {
        assert_eq!(
            CompressionAlgorithm::from_str("zstd").unwrap(),
            CompressionAlgorithm::Zstd
        );
        assert_eq!(
            CompressionAlgorithm::from_str("lz4").unwrap(),
            CompressionAlgorithm::Lz4
        );
        assert_eq!(
            CompressionAlgorithm::from_str("brotli").unwrap(),
            CompressionAlgorithm::Brotli
        );
        assert_eq!(
            CompressionAlgorithm::from_str("deflate").unwrap(),
            CompressionAlgorithm::Deflate
        );
        assert_eq!(
            CompressionAlgorithm::from_str("gzip").unwrap(),
            CompressionAlgorithm::Gzip
        );
    }

    #[test]
    fn test_compression_algorithm_from_str_case_insensitive() {
        assert_eq!(
            CompressionAlgorithm::from_str("ZSTD").unwrap(),
            CompressionAlgorithm::Zstd
        );
        assert_eq!(
            CompressionAlgorithm::from_str("Lz4").unwrap(),
            CompressionAlgorithm::Lz4
        );
        assert_eq!(
            CompressionAlgorithm::from_str("BrOtLi").unwrap(),
            CompressionAlgorithm::Brotli
        );
    }

    #[test]
    fn test_compression_algorithm_from_str_invalid() {
        assert!(CompressionAlgorithm::from_str("invalid").is_err());
        assert!(CompressionAlgorithm::from_str("foobar").is_err());
        assert!(CompressionAlgorithm::from_str("").is_err());
    }

    #[test]
    fn test_compression_algorithm_display() {
        assert_eq!(CompressionAlgorithm::Zstd.to_string(), "zstd");
        assert_eq!(CompressionAlgorithm::Lz4.to_string(), "lz4");
        assert_eq!(CompressionAlgorithm::Brotli.to_string(), "brotli");
        assert_eq!(CompressionAlgorithm::Deflate.to_string(), "deflate");
        assert_eq!(CompressionAlgorithm::Gzip.to_string(), "gzip");
    }

    #[test]
    fn test_compression_algorithm_debug() {
        let algo = CompressionAlgorithm::Zstd;
        let debug_str = format!("{:?}", algo);
        assert!(debug_str.contains("Zstd"));
    }

    #[test]
    fn test_compression_algorithm_serialization() {
        let algorithms = vec![
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Brotli,
            CompressionAlgorithm::Deflate,
            CompressionAlgorithm::Gzip,
        ];

        for algo in algorithms {
            let serialized = serde_json::to_string(&algo).unwrap();
            let deserialized: CompressionAlgorithm = serde_json::from_str(&serialized).unwrap();
            assert_eq!(algo, deserialized);
        }
    }

    #[test]
    fn test_compression_algorithm_equality() {
        assert_eq!(CompressionAlgorithm::Zstd, CompressionAlgorithm::Zstd);
        assert_ne!(CompressionAlgorithm::Zstd, CompressionAlgorithm::Lz4);
    }

    #[test]
    fn test_compression_algorithm_clone() {
        let original = CompressionAlgorithm::Zstd;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_get_compressor_zstd() {
        let _compressor = get_compressor(CompressionAlgorithm::Zstd);
        // Verify it returns a valid compressor trait object
        let _trait_obj: &dyn CompressionTrait = &*_compressor;
    }

    #[test]
    fn test_get_compressor_lz4() {
        let _compressor = get_compressor(CompressionAlgorithm::Lz4);
        let _trait_obj: &dyn CompressionTrait = &*_compressor;
    }

    #[test]
    fn test_get_compressor_brotli() {
        let _compressor = get_compressor(CompressionAlgorithm::Brotli);
        let _trait_obj: &dyn CompressionTrait = &*_compressor;
    }

    #[test]
    fn test_get_compressor_deflate() {
        let _compressor = get_compressor(CompressionAlgorithm::Deflate);
        let _trait_obj: &dyn CompressionTrait = &*_compressor;
    }

    #[test]
    fn test_get_compressor_gzip() {
        let _compressor = get_compressor(CompressionAlgorithm::Gzip);
        let _trait_obj: &dyn CompressionTrait = &*_compressor;
    }

    #[tokio::test]
    async fn test_compression_roundtrip_zstd() {
        let compressor = get_compressor(CompressionAlgorithm::Zstd);
        let original_data = b"Hello, World! This is test data for compression roundtrip testing.";

        let compressed = compressor.compress(original_data).await.unwrap();
        assert!(compressed.len() > 0);

        let decompressed = compressor.decompress(&compressed).await.unwrap();
        assert_eq!(decompressed, original_data);
    }

    #[tokio::test]
    async fn test_compression_roundtrip_lz4() {
        let compressor = get_compressor(CompressionAlgorithm::Lz4);
        let original_data = b"Hello, World! This is test data for compression roundtrip testing.";

        let compressed = compressor.compress(original_data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();
        assert_eq!(decompressed, original_data);
    }

    #[tokio::test]
    async fn test_compression_roundtrip_brotli() {
        let compressor = get_compressor(CompressionAlgorithm::Brotli);
        let original_data = b"Hello, World! This is test data for compression roundtrip testing.";

        let compressed = compressor.compress(original_data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();
        assert_eq!(decompressed, original_data);
    }

    #[tokio::test]
    async fn test_compression_roundtrip_deflate() {
        let compressor = get_compressor(CompressionAlgorithm::Deflate);
        let original_data = b"Hello, World! This is test data for compression roundtrip testing.";

        let compressed = compressor.compress(original_data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();
        assert_eq!(decompressed, original_data);
    }

    #[tokio::test]
    async fn test_compression_roundtrip_gzip() {
        let compressor = get_compressor(CompressionAlgorithm::Gzip);
        let original_data = b"Hello, World! This is test data for compression roundtrip testing.";

        let compressed = compressor.compress(original_data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();
        assert_eq!(decompressed, original_data);
    }

    #[tokio::test]
    async fn test_compression_empty_data() {
        let algorithms = vec![
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Brotli,
            CompressionAlgorithm::Deflate,
            CompressionAlgorithm::Gzip,
        ];

        for algo in algorithms {
            let compressor = get_compressor(algo);
            let empty_data = b"";

            let compressed = compressor.compress(empty_data).await.unwrap();
            let decompressed = compressor.decompress(&compressed).await.unwrap();
            assert_eq!(decompressed, empty_data);
        }
    }

    #[tokio::test]
    async fn test_compression_large_data() {
        let compressor = get_compressor(CompressionAlgorithm::Zstd);
        let original_data: Vec<u8> = (0 .. 10000).map(|i| (i % 256) as u8).collect();

        let compressed = compressor.compress(&original_data).await.unwrap();
        // Compression should reduce size for repetitive data
        assert!(compressed.len() < original_data.len());

        let decompressed = compressor.decompress(&compressed).await.unwrap();
        assert_eq!(decompressed, original_data);
    }

    #[test]
    fn test_compression_algorithm_all_variants_covered() {
        // Ensure all enum variants are covered
        let algorithms = vec![
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Brotli,
            CompressionAlgorithm::Deflate,
            CompressionAlgorithm::Gzip,
        ];

        for algo in algorithms {
            match algo {
                CompressionAlgorithm::Zstd => assert_eq!(algo.to_string(), "zstd"),
                CompressionAlgorithm::Lz4 => assert_eq!(algo.to_string(), "lz4"),
                CompressionAlgorithm::Brotli => assert_eq!(algo.to_string(), "brotli"),
                CompressionAlgorithm::Deflate => assert_eq!(algo.to_string(), "deflate"),
                CompressionAlgorithm::Gzip => assert_eq!(algo.to_string(), "gzip"),
            }
        }
    }
}
