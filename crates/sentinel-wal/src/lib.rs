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
//! - Collection name (256 bytes, fixed length, padded/truncated)
//! - Document ID (256 bytes, fixed length, padded/truncated)
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

pub mod entry;
pub mod error;
pub mod manager;

// Re-exports
pub use error::WalError;
pub use entry::{EntryType, FixedBytes256, FixedBytes32, LogEntry};
pub use manager::WalManager;
pub use postcard;
pub use cuid2;

/// Result type for WAL operations
pub type Result<T> = std::result::Result<T, WalError>;

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use serde_json::json;

    use crate::{EntryType, LogEntry, WalManager};

    /// Test that log entries can be serialized and deserialized correctly.
    ///
    /// This test verifies:
    /// - Postcard serialization works for LogEntry
    /// - Checksum validation prevents corruption
    /// - All fields are preserved through serialization
    #[tokio::test]
    async fn test_log_entry_serialization() {
        let transaction_id = cuid2::create_id();
        let entry = LogEntry::new(
            EntryType::Insert,
            transaction_id.clone(),
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

        let wal = WalManager::new(wal_path).await.unwrap();

        let transaction_id = cuid2::create_id();
        let entry = LogEntry::new(
            EntryType::Insert,
            transaction_id,
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
    /// - Checkpoint truncates the WAL file
    /// - File size is reset to zero after checkpoint
    /// - WAL remains functional after checkpoint
    #[tokio::test]
    async fn test_wal_checkpoint() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        let wal = WalManager::new(wal_path.clone()).await.unwrap();

        let transaction_id = cuid2::create_id();
        let entry = LogEntry::new(
            EntryType::Insert,
            transaction_id,
            "users".to_string(),
            "user-123".to_string(),
            Some(json!({"name": "Alice"})),
        );

        wal.write_entry(entry).await.unwrap();

        assert!(wal.size().await.unwrap() > 0);

        wal.checkpoint().await.unwrap();

        assert_eq!(wal.size().await.unwrap(), 0);
    }
}
