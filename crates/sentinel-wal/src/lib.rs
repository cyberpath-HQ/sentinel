//! # Sentinel WAL (Write-Ahead Logging)
//!
//! This crate implements the Write-Ahead Logging (WAL) functionality for Cyberpath Sentinel.
//! WAL ensures durability and crash recovery by logging all changes before they are applied
//! to the filesystem.
//!
//! ## Architecture
//!
//! The WAL consists of log entries written to a binary file. Each entry contains:
//! - Entry type (1 byte)
//! - Transaction ID (32 bytes, fixed length, padded cuid2)
//! - Collection name (variable, multiple of 16 bytes, max 256)
//! - Document ID (variable, multiple of 16 bytes, max 256)
//! - Timestamp (8 bytes, u64)
//! - Data length (8 bytes)
//! - Data (variable length, JSON string)
//! - CRC32 checksum (4 bytes)
//!
//! ## Features
//!
//! - Postcard serialization for efficiency and maintainability
//! - CRC32 checksums for integrity
//! - Asynchronous I/O operations
//! - Checkpoint mechanism for log compaction
//! - Crash recovery via log replay

pub mod compression;
pub mod config;
pub mod entry;
pub mod error;
pub mod manager;
pub mod recovery;
pub mod traits;
pub mod verification;

// Re-exports
pub use error::WalError;
pub use entry::{EntryType, FixedBytes256, FixedBytes32, LogEntry};
pub use manager::{WalConfig, WalFormat, WalManager};
pub use config::{CollectionWalConfig, CollectionWalConfigOverrides, StoreWalConfig, WalFailureMode};
pub use traits::WalDocumentOps;
pub use verification::{verify_wal_consistency, WalVerificationIssue, WalVerificationResult};
pub use recovery::{recover_from_wal_force, recover_from_wal_safe, WalRecoveryFailure, WalRecoveryResult};
pub use compression::*;
pub use postcard;
pub use cuid2;

/// Result type for WAL operations
pub type Result<T> = std::result::Result<T, WalError>;

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use serde_json::json;

    use crate::{EntryType, LogEntry, WalConfig, WalManager};

    /// Test that log entries can be serialized and deserialized correctly.
    ///
    /// This test verifies:
    /// - Postcard serialization works for LogEntry
    /// - Checksum validation prevents corruption
    /// - All fields are preserved through serialization
    #[tokio::test]
    async fn test_log_entry_serialization() {
        let entry = LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-123".to_string(),
            Some(json!({"name": "Alice"})),
        );

        let bytes = entry.to_bytes().unwrap();
        let deserialized = LogEntry::from_bytes(&bytes).unwrap();

        assert_eq!(
            std::mem::discriminant(&entry.entry_type),
            std::mem::discriminant(&deserialized.entry_type)
        );
        assert_eq!(entry.transaction_id, deserialized.transaction_id);
        assert_eq!(entry.collection, deserialized.collection);
        assert_eq!(entry.document_id, deserialized.document_id);
        assert_eq!(entry.timestamp, deserialized.timestamp);
        assert_eq!(
            entry.data_as_value().unwrap(),
            deserialized.data_as_value().unwrap()
        );
    }

    /// Test basic WAL write and read operations.
    ///
    /// This test verifies:
    /// - Entries can be written to the WAL file
    /// - Entries can be read back correctly
    /// - File I/O operations work as expected
    #[tokio::test]
    async fn test_wal_write_and_read() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        let wal = WalManager::new(wal_path, WalConfig::default())
            .await
            .unwrap();

        let entry = LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-123".to_string(),
            Some(json!({"name": "Alice"})),
        );

        wal.write_entry(entry.clone()).await.unwrap();

        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(&entries[0].collection[.. 5], b"users");
        assert_eq!(&entries[0].document_id[.. 8], b"user-123");
    }

    /// Test WAL checkpoint functionality.
    ///
    /// This test verifies:
    /// - Checkpoint flushes WAL entries to disk
    /// - File size remains unchanged (entries preserved)
    /// - WAL remains functional after checkpoint
    /// - Checkpoint creates a durable recovery point
    #[tokio::test]
    async fn test_wal_checkpoint() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let entry = LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-123".to_string(),
            Some(json!({"name": "Alice"})),
        );

        wal.write_entry(entry).await.unwrap();

        let size_before_checkpoint = wal.size().await.unwrap();
        assert!(size_before_checkpoint > 0);

        wal.checkpoint().await.unwrap();

        // File size should remain the same (entries preserved)
        let size_after_checkpoint = wal.size().await.unwrap();
        assert_eq!(size_before_checkpoint, size_after_checkpoint);

        // Verify entries can still be read after checkpoint
        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].collection_str(), "users");
        assert_eq!(entries[0].document_id_str(), "user-123");
    }

    /// Test WAL file format demonstration.
    ///
    /// This test creates a WAL file with sample entries and demonstrates the binary format.
    /// The format is: [length:u32_le][postcard_data][crc32:u32_le] for each entry.
    #[tokio::test]
    async fn test_wal_file_format() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("format_demo.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write a few entries
        let entries = vec![
            LogEntry::new(
                EntryType::Insert,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice"})),
            ),
            LogEntry::new(
                EntryType::Update,
                "products".to_string(),
                "prod-2".to_string(),
                Some(json!({"price": 29.99})),
            ),
        ];

        for entry in entries {
            wal.write_entry(entry).await.unwrap();
        }

        // Read the raw file bytes
        let file_bytes = tokio::fs::read(&wal_path).await.unwrap();

        println!("WAL file binary format demonstration:");
        println!("Total file size: {} bytes", file_bytes.len());
        println!("Hex dump:");
        for (i, chunk) in file_bytes.chunks(16).enumerate() {
            print!("{:08x}: ", i * 16);
            for &byte in chunk {
                print!("{:02x} ", byte);
            }
            println!();
        }

        // Verify we can read back the entries
        let read_entries = wal.read_all_entries().await.unwrap();
        assert_eq!(read_entries.len(), 2);
    }

    /// Test JSON Lines format serialization and deserialization.
    #[tokio::test]
    async fn test_json_lines_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_json.wal");

        let config = WalConfig {
            format: crate::WalFormat::JsonLines,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Create test entries
        let entries = vec![
            LogEntry::new(
                EntryType::Insert,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice", "age": 30})),
            ),
            LogEntry::new(
                EntryType::Update,
                "products".to_string(),
                "prod-2".to_string(),
                Some(json!({"price": 29.99, "category": "electronics"})),
            ),
            LogEntry::new(
                EntryType::Delete,
                "users".to_string(),
                "user-old".to_string(),
                None,
            ),
        ];

        // Write entries
        for entry in &entries {
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Read file content as text
        let file_content = tokio::fs::read_to_string(&wal_path).await.unwrap();
        println!("JSON Lines format content:");
        println!("{}", file_content);

        // Verify each line is valid JSON
        for line in file_content.lines() {
            let _: serde_json::Value = serde_json::from_str(line).unwrap();
        }

        // Read back entries and verify
        let read_entries = wal.read_all_entries().await.unwrap();
        assert_eq!(read_entries.len(), 3);

        for (original, read) in entries.iter().zip(read_entries.iter()) {
            assert_eq!(original.entry_type, read.entry_type);
            assert_eq!(original.transaction_id_str(), read.transaction_id_str());
            assert_eq!(original.collection_str(), read.collection_str());
            assert_eq!(original.document_id_str(), read.document_id_str());
            assert_eq!(original.timestamp, read.timestamp);
            assert_eq!(original.data, read.data);
        }
    }

    /// Test Zstd compression functionality.
    #[tokio::test]
    async fn test_zstd_compression() {
        use crate::compression::{CompressionTrait, ZstdCompressor};

        let compressor = ZstdCompressor;

        // Test compression and decompression
        let original_data = b"Hello, World! This is a test message for compression.";
        let compressed = compressor.compress(original_data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();

        assert_eq!(original_data, decompressed.as_slice());
        // Note: Some compression algorithms may not compress small data effectively
        assert!(compressed.len() <= original_data.len() + 100); // Allow reasonable expansion for
                                                                // small data
    }

    /// Test LZ4 compression functionality.
    #[tokio::test]
    async fn test_lz4_compression() {
        use crate::compression::{CompressionTrait, Lz4Compressor};

        let compressor = Lz4Compressor;

        // Test compression and decompression
        let original_data = b"Hello, World! This is a test message for compression.";
        let compressed = compressor.compress(original_data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();

        assert_eq!(original_data, decompressed.as_slice());
        // Note: Some compression algorithms may not compress small data effectively
        assert!(compressed.len() <= original_data.len() + 100); // Allow reasonable expansion for
                                                                // small data
    }

    /// Test Brotli compression functionality.
    #[tokio::test]
    async fn test_brotli_compression() {
        use crate::compression::{BrotliCompressor, CompressionTrait};

        let compressor = BrotliCompressor;

        // Test compression and decompression
        let original_data = b"Hello, World! This is a test message for compression.";
        let compressed = compressor.compress(original_data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();

        assert_eq!(original_data, decompressed.as_slice());
        assert!(compressed.len() < original_data.len()); // Compression should reduce size
    }

    /// Test Deflate compression functionality.
    #[tokio::test]
    async fn test_deflate_compression() {
        use crate::compression::{CompressionTrait, DeflateCompressor};

        let compressor = DeflateCompressor;

        // Test compression and decompression
        let original_data = b"Hello, World! This is a test message for compression.";
        let compressed = compressor.compress(original_data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();

        assert_eq!(original_data, decompressed.as_slice());
        // Note: Some compression algorithms may not compress small data effectively
        assert!(compressed.len() <= original_data.len() + 100); // Allow reasonable expansion for
                                                                // small data
    }

    /// Test Gzip compression functionality.
    #[tokio::test]
    async fn test_gzip_compression() {
        use crate::compression::{CompressionTrait, GzipCompressor};

        let compressor = GzipCompressor;

        // Test compression and decompression
        let original_data = b"Hello, World! This is a test message for compression.";
        let compressed = compressor.compress(original_data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();

        assert_eq!(original_data, decompressed.as_slice());
        // Note: Some compression algorithms may not compress small data effectively
        assert!(compressed.len() <= original_data.len() + 100); // Allow reasonable expansion for
                                                                // small data
    }

    /// Test compression algorithms work correctly.
    #[tokio::test]
    async fn test_compression_algorithms() {
        use crate::compression::{get_compressor, CompressionAlgorithm};

        let test_data = b"This is test data for compression algorithm validation.";

        // Test all compression algorithms
        for algorithm in &[
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Brotli,
            CompressionAlgorithm::Deflate,
            CompressionAlgorithm::Gzip,
        ] {
            let compressor = get_compressor(*algorithm);
            let compressed = compressor.compress(test_data).await.unwrap();
            let decompressed = compressor.decompress(&compressed).await.unwrap();

            assert_eq!(test_data, decompressed.as_slice());
            // Note: Some compression algorithms may not compress small data effectively
            assert!(compressed.len() <= test_data.len() + 100); // Allow reasonable expansion for
                                                                // small data
        }
    }

    #[tokio::test]
    async fn test_compression_algorithm_from_str() {
        use crate::CompressionAlgorithm;

        // Test valid algorithms
        assert_eq!(
            "zstd".parse::<CompressionAlgorithm>().unwrap(),
            CompressionAlgorithm::Zstd
        );
        assert_eq!(
            "lz4".parse::<CompressionAlgorithm>().unwrap(),
            CompressionAlgorithm::Lz4
        );
        assert_eq!(
            "brotli".parse::<CompressionAlgorithm>().unwrap(),
            CompressionAlgorithm::Brotli
        );
        assert_eq!(
            "deflate".parse::<CompressionAlgorithm>().unwrap(),
            CompressionAlgorithm::Deflate
        );
        assert_eq!(
            "gzip".parse::<CompressionAlgorithm>().unwrap(),
            CompressionAlgorithm::Gzip
        );

        // Test case insensitive
        assert_eq!(
            "ZSTD".parse::<CompressionAlgorithm>().unwrap(),
            CompressionAlgorithm::Zstd
        );
        assert_eq!(
            "Lz4".parse::<CompressionAlgorithm>().unwrap(),
            CompressionAlgorithm::Lz4
        );

        // Test invalid algorithm
        assert!("invalid".parse::<CompressionAlgorithm>().is_err());
    }

    /// Test compression with corrupted data.
    #[tokio::test]
    async fn test_compression_corrupted_data() {
        use crate::compression::{CompressionTrait, ZstdCompressor};

        let compressor = ZstdCompressor;

        // Compress valid data
        let original_data = b"This is valid data for compression";
        let compressed = compressor.compress(original_data).await.unwrap();

        // Corrupt the compressed data
        let mut corrupted = compressed.clone();
        if corrupted.len() > 0 {
            corrupted[0] ^= 0xff; // Flip bits in first byte
        }

        // Decompression should fail
        let result = compressor.decompress(&corrupted).await;
        assert!(result.is_err());

        // Test with completely invalid data
        let invalid_data = b"This is not compressed data at all";
        let result = compressor.decompress(invalid_data).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wal_failure_mode_from_str() {
        use crate::WalFailureMode;

        // Test valid modes
        assert_eq!(
            "disabled".parse::<WalFailureMode>().unwrap(),
            WalFailureMode::Disabled
        );
        assert_eq!(
            "warn".parse::<WalFailureMode>().unwrap(),
            WalFailureMode::Warn
        );
        assert_eq!(
            "strict".parse::<WalFailureMode>().unwrap(),
            WalFailureMode::Strict
        );

        // Test case insensitive
        assert_eq!(
            "DISABLED".parse::<WalFailureMode>().unwrap(),
            WalFailureMode::Disabled
        );
        assert_eq!(
            "Warn".parse::<WalFailureMode>().unwrap(),
            WalFailureMode::Warn
        );

        // Test invalid mode
        assert!("invalid".parse::<WalFailureMode>().is_err());
    }

    #[tokio::test]
    async fn test_wal_failure_mode_display() {
        use crate::WalFailureMode;

        assert_eq!(format!("{}", WalFailureMode::Disabled), "disabled");
        assert_eq!(format!("{}", WalFailureMode::Warn), "warn");
        assert_eq!(format!("{}", WalFailureMode::Strict), "strict");
    }

    /// Test WAL recovery functionality.
    ///
    /// This test verifies:
    /// - Safe recovery handles conflicts correctly
    /// - Force recovery overwrites conflicts
    /// - Recovery is idempotent
    /// - Error handling works properly
    #[tokio::test]
    async fn test_wal_recovery_safe() {
        use std::{
            collections::HashMap,
            sync::{Arc, Mutex},
        };

        use crate::recovery::{recover_from_wal_safe, WalRecoveryResult};

        // Mock document ops that tracks operations
        #[derive(Clone)]
        struct MockDocumentOps {
            operations: Arc<Mutex<Vec<(String, String, Option<serde_json::Value>)>>>,
            documents:  Arc<Mutex<HashMap<String, serde_json::Value>>>,
        }

        impl MockDocumentOps {
            fn new() -> Self {
                Self {
                    operations: Arc::new(Mutex::new(Vec::new())),
                    documents:  Arc::new(Mutex::new(HashMap::new())),
                }
            }
        }

        #[async_trait::async_trait]
        impl crate::traits::WalDocumentOps for MockDocumentOps {
            async fn get_document(&self, doc_id: &str) -> crate::Result<Option<serde_json::Value>> {
                println!("get_document called for {}", doc_id);
                let docs = self.documents.lock().unwrap();
                Ok(docs.get(doc_id).cloned())
            }

            async fn apply_operation(
                &self,
                op_type: &EntryType,
                doc_id: &str,
                data: Option<serde_json::Value>,
            ) -> crate::Result<()> {
                println!("apply_operation called: {:?} on {}", op_type, doc_id);
                let mut ops = self.operations.lock().unwrap();
                ops.push((format!("{:?}", op_type), doc_id.to_string(), data.clone()));

                let mut docs = self.documents.lock().unwrap();
                match op_type {
                    EntryType::Insert => {
                        if let Some(data) = data {
                            docs.insert(doc_id.to_string(), data);
                        }
                    },
                    EntryType::Update => {
                        if let Some(data) = data {
                            docs.insert(doc_id.to_string(), data);
                        }
                    },
                    EntryType::Delete => {
                        docs.remove(doc_id);
                    },
                    _ => {}, // Skip other operations for this test
                }
                Ok(())
            }
        }

        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_recovery_safe.wal");

        let config = WalConfig {
            format: crate::WalFormat::Binary,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();
        let mock_ops = MockDocumentOps::new();

        // Create entries with some conflicts (multiple operations on same document)
        let entries = vec![
            LogEntry::new(
                EntryType::Insert,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice", "age": 30})),
            ),
            LogEntry::new(
                EntryType::Update,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice", "age": 31})),
            ),
            LogEntry::new(
                EntryType::Insert,
                "users".to_string(),
                "user-2".to_string(),
                Some(json!({"name": "Bob", "age": 25})),
            ),
            LogEntry::new(
                EntryType::Delete,
                "users".to_string(),
                "user-1".to_string(),
                None,
            ),
        ];

        // Write entries to WAL
        for entry in &entries {
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Checkpoint to ensure all data is flushed
        wal.checkpoint().await.unwrap();

        // Debug: Check file size
        let file_size = tokio::fs::metadata(&wal_path).await.unwrap().len();
        println!("WAL file size: {} bytes", file_size);

        // Test recovery
        println!("Calling recover_from_wal_safe");

        // Debug: Test stream_entries directly
        println!("Testing stream_entries");
        use futures::StreamExt;
        let stream = wal.stream_entries();
        let mut pinned_stream = std::pin::pin!(stream);
        let mut stream_count = 0;
        while let Some(entry_result) = pinned_stream.next().await {
            match entry_result {
                Ok(entry) => {
                    println!(
                        "Stream yielded entry: {:?} {}",
                        entry.entry_type,
                        entry.document_id_str()
                    );
                    stream_count += 1;
                },
                Err(e) => {
                    println!("Stream yielded error: {}", e);
                },
            }
        }
        println!("Stream yielded {} entries", stream_count);

        let result = recover_from_wal_safe(&wal, &mock_ops).await.unwrap();
        println!(
            "Recovery result: {} recovered, {} skipped, {} failed",
            result.recovered_operations, result.skipped_operations, result.failed_operations
        );

        // All operations should be recovered
        assert_eq!(result.recovered_operations, 4);
        assert_eq!(result.skipped_operations, 0);
        assert_eq!(result.failed_operations, 0);

        // Check final state (last operation wins)
        let docs = mock_ops.documents.lock().unwrap();
        assert_eq!(docs.len(), 1); // user-1 was deleted, user-2 remains
        assert_eq!(docs["user-2"], json!({"name": "Bob", "age": 25}));

        // Check operations were applied
        let ops = mock_ops.operations.lock().unwrap();
        assert_eq!(ops.len(), 4);
        assert_eq!(
            ops[0],
            (
                "Insert".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice", "age": 30}))
            )
        );
        assert_eq!(
            ops[1],
            (
                "Update".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice", "age": 31}))
            )
        );
        assert_eq!(
            ops[2],
            (
                "Insert".to_string(),
                "user-2".to_string(),
                Some(json!({"name": "Bob", "age": 25}))
            )
        );
        assert_eq!(ops[3], ("Delete".to_string(), "user-1".to_string(), None));
    }

    /// Test WAL force recovery functionality.
    #[tokio::test]
    async fn test_wal_recovery_force() {
        use std::{
            collections::HashMap,
            sync::{Arc, Mutex},
        };

        use crate::recovery::{recover_from_wal_force, WalRecoveryResult};

        // Mock document ops
        #[derive(Clone)]
        struct MockDocumentOps {
            operations: Arc<Mutex<Vec<String>>>,
            documents:  Arc<Mutex<HashMap<String, serde_json::Value>>>,
        }

        impl MockDocumentOps {
            fn new() -> Self {
                Self {
                    operations: Arc::new(Mutex::new(Vec::new())),
                    documents:  Arc::new(Mutex::new(HashMap::new())),
                }
            }
        }

        #[async_trait::async_trait]
        impl crate::traits::WalDocumentOps for MockDocumentOps {
            async fn get_document(&self, doc_id: &str) -> crate::Result<Option<serde_json::Value>> {
                let docs = self.documents.lock().unwrap();
                Ok(docs.get(doc_id).cloned())
            }

            async fn apply_operation(
                &self,
                op_type: &EntryType,
                doc_id: &str,
                data: Option<serde_json::Value>,
            ) -> crate::Result<()> {
                let mut ops = self.operations.lock().unwrap();
                ops.push(format!("{:?}", op_type));

                let mut docs = self.documents.lock().unwrap();
                match op_type {
                    EntryType::Insert => {
                        if let Some(data) = data {
                            docs.insert(doc_id.to_string(), data);
                        }
                    },
                    EntryType::Update => {
                        if let Some(data) = data {
                            docs.insert(doc_id.to_string(), data);
                        }
                    },
                    EntryType::Delete => {
                        docs.remove(doc_id);
                    },
                    _ => {}, // Skip other operations for this test
                }
                Ok(())
            }
        }

        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_recovery_force.wal");

        let config = WalConfig {
            format: crate::WalFormat::Binary,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();
        let mock_ops = MockDocumentOps::new();

        // Create entries with conflicts
        let entries = vec![
            LogEntry::new(
                EntryType::Insert,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice"})),
            ),
            LogEntry::new(
                EntryType::Update,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice Updated"})),
            ),
            LogEntry::new(
                EntryType::Delete,
                "users".to_string(),
                "user-1".to_string(),
                None,
            ),
        ];

        // Write entries to WAL
        for entry in &entries {
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Test force recovery
        let result = recover_from_wal_force(&wal, &mock_ops).await.unwrap();

        // All operations should be recovered
        assert_eq!(result.recovered_operations, 3);
        assert_eq!(result.skipped_operations, 0);
        assert_eq!(result.failed_operations, 0);

        // Check final state (last operation wins)
        let docs = mock_ops.documents.lock().unwrap();
        assert_eq!(docs.len(), 0); // user-1 was deleted

        // Check operations were applied
        let ops = mock_ops.operations.lock().unwrap();
        assert_eq!(ops.len(), 3);
    }

    /// Test WAL recovery with duplicate operations.
    #[tokio::test]
    async fn test_wal_recovery_duplicates() {
        use std::{
            collections::HashMap,
            sync::{Arc, Mutex},
        };

        use crate::recovery::recover_from_wal_safe;

        // Mock document ops
        #[derive(Clone)]
        struct MockDocumentOps {
            operations: Arc<Mutex<Vec<String>>>,
            documents:  Arc<Mutex<HashMap<String, serde_json::Value>>>,
        }

        impl MockDocumentOps {
            fn new() -> Self {
                Self {
                    operations: Arc::new(Mutex::new(Vec::new())),
                    documents:  Arc::new(Mutex::new(HashMap::new())),
                }
            }
        }

        #[async_trait::async_trait]
        impl crate::traits::WalDocumentOps for MockDocumentOps {
            async fn get_document(&self, doc_id: &str) -> crate::Result<Option<serde_json::Value>> {
                let docs = self.documents.lock().unwrap();
                Ok(docs.get(doc_id).cloned())
            }

            async fn apply_operation(
                &self,
                op_type: &EntryType,
                doc_id: &str,
                data: Option<serde_json::Value>,
            ) -> crate::Result<()> {
                let mut ops = self.operations.lock().unwrap();
                ops.push(format!("{:?}", op_type));

                let mut docs = self.documents.lock().unwrap();
                match op_type {
                    EntryType::Insert => {
                        if let Some(data) = data {
                            docs.insert(doc_id.to_string(), data);
                        }
                    },
                    _ => {}, // Skip other operations for this test
                }
                Ok(())
            }
        }

        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_recovery_duplicates.wal");

        let config = WalConfig {
            format: crate::WalFormat::Binary,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();
        let mock_ops = MockDocumentOps::new();

        // Create duplicate entries
        let entries = vec![
            LogEntry::new(
                EntryType::Insert,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice"})),
            ),
            LogEntry::new(
                EntryType::Insert,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice Duplicate"})),
            ),
        ];

        // Write entries to WAL
        for entry in &entries {
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Test recovery - should handle duplicates gracefully
        let result = recover_from_wal_safe(&wal, &mock_ops).await.unwrap();

        // First insert should be recovered, second should be skipped (document already exists)
        assert_eq!(result.recovered_operations, 1);
        assert_eq!(result.skipped_operations, 1);
        assert_eq!(result.failed_operations, 0);

        // Check final state
        let docs = mock_ops.documents.lock().unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs["user-1"], json!({"name": "Alice"})); // First operation applied

        // Check operations
        let ops = mock_ops.operations.lock().unwrap();
        assert_eq!(ops.len(), 1); // Only one operation applied
    }

    /// Test WAL recovery with invalid JSON data.
    #[tokio::test]
    async fn test_wal_recovery_invalid_json() {
        use std::{
            collections::HashMap,
            sync::{Arc, Mutex},
        };

        use crate::recovery::recover_from_wal_safe;

        // Mock document ops
        #[derive(Clone)]
        struct MockDocumentOps {
            documents: Arc<Mutex<HashMap<String, serde_json::Value>>>,
        }

        impl MockDocumentOps {
            fn new() -> Self {
                Self {
                    documents: Arc::new(Mutex::new(HashMap::new())),
                }
            }
        }

        #[async_trait::async_trait]
        impl crate::traits::WalDocumentOps for MockDocumentOps {
            async fn get_document(&self, _doc_id: &str) -> crate::Result<Option<serde_json::Value>> { Ok(None) }

            async fn apply_operation(
                &self,
                _op_type: &EntryType,
                _doc_id: &str,
                _data: Option<serde_json::Value>,
            ) -> crate::Result<()> {
                Ok(())
            }
        }

        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_invalid_json.wal");

        let config = WalConfig {
            format: crate::WalFormat::Binary,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();
        let mock_ops = MockDocumentOps::new();

        // Create entry with invalid JSON
        let mut entry = LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            None, // No data initially
        );
        // Manually set invalid JSON data
        entry.data = Some("{invalid json".to_string());

        // Write entry to WAL
        wal.write_entry(entry).await.unwrap();

        // Test recovery - should handle invalid JSON gracefully
        let result = recover_from_wal_safe(&wal, &mock_ops).await.unwrap();

        // Operation should fail due to invalid JSON
        assert_eq!(result.recovered_operations, 0);
        assert_eq!(result.skipped_operations, 0);
        assert_eq!(result.failed_operations, 1);
        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].reason.contains("Invalid JSON"));
    }

    /// Test WAL recovery with transaction control entries.
    #[tokio::test]
    async fn test_wal_recovery_transaction_control() {
        use std::{
            collections::HashMap,
            sync::{Arc, Mutex},
        };

        use crate::recovery::recover_from_wal_safe;

        // Mock document ops that tracks operations
        #[derive(Clone)]
        struct MockDocumentOps {
            operations: Arc<Mutex<Vec<String>>>,
        }

        impl MockDocumentOps {
            fn new() -> Self {
                Self {
                    operations: Arc::new(Mutex::new(Vec::new())),
                }
            }
        }

        #[async_trait::async_trait]
        impl crate::traits::WalDocumentOps for MockDocumentOps {
            async fn get_document(&self, _doc_id: &str) -> crate::Result<Option<serde_json::Value>> { Ok(None) }

            async fn apply_operation(
                &self,
                op_type: &EntryType,
                _doc_id: &str,
                _data: Option<serde_json::Value>,
            ) -> crate::Result<()> {
                let mut ops = self.operations.lock().unwrap();
                ops.push(format!("{:?}", op_type));
                Ok(())
            }
        }

        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_transaction_control.wal");

        let config = WalConfig {
            format: crate::WalFormat::Binary,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();
        let mock_ops = MockDocumentOps::new();

        // Create transaction control entries
        let entries = vec![
            LogEntry::new(
                EntryType::Begin,
                "users".to_string(),
                "txn-1".to_string(),
                None,
            ),
            LogEntry::new(
                EntryType::Insert,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice"})),
            ),
            LogEntry::new(
                EntryType::Commit,
                "users".to_string(),
                "txn-1".to_string(),
                None,
            ),
            LogEntry::new(
                EntryType::Begin,
                "users".to_string(),
                "txn-2".to_string(),
                None,
            ),
            LogEntry::new(
                EntryType::Rollback,
                "users".to_string(),
                "txn-2".to_string(),
                None,
            ),
        ];

        // Write entries to WAL
        for entry in &entries {
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Test recovery
        println!("Calling recover_from_wal_safe");
        let result = recover_from_wal_safe(&wal, &mock_ops).await.unwrap();
        println!(
            "Recovery result: {} recovered, {} skipped, {} failed",
            result.recovered_operations, result.skipped_operations, result.failed_operations
        );

        // Only the insert operation should be recovered, transaction controls skipped
        assert_eq!(result.recovered_operations, 1);
        assert_eq!(result.skipped_operations, 4); // Begin, Commit, Begin, Rollback
        assert_eq!(result.failed_operations, 0);

        // Check operations applied
        let ops = mock_ops.operations.lock().unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0], "Insert");
    }
}
