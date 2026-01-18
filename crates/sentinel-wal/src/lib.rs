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
pub mod entry;
pub mod error;
pub mod manager;

// Re-exports
pub use error::WalError;
pub use entry::{EntryType, FixedBytes256, FixedBytes32, LogEntry};
pub use manager::{WalConfig, WalFormat, WalManager};
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
}
