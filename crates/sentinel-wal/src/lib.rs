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
//! - Transaction ID (16 bytes, UUID)
//! - Collection name (32 bytes, padded string)
//! - Document ID (256 bytes, padded string)
//! - Data length (8 bytes)
//! - Data (variable length, JSON)
//! - CRC32 checksum (4 bytes)
//!
//! ## Features
//!
//! - Binary serialization for efficiency
//! - CRC32 checksums for integrity
//! - Asynchronous I/O operations
//! - Checkpoint mechanism for log compaction
//! - Crash recovery via log replay

use std::{path::PathBuf, sync::Arc};

use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom},
};
use serde::{Deserialize, Serialize};
use bincode;
use crc32fast::Hasher as Crc32Hasher;
use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Error types for WAL operations
#[derive(thiserror::Error, Debug)]
pub enum WalError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("Invalid log entry: {0}")]
    InvalidEntry(String),
    #[error("Checksum mismatch")]
    ChecksumMismatch,
}

/// Result type for WAL operations
pub type Result<T> = std::result::Result<T, WalError>;

/// Types of WAL entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryType {
    /// Begin a transaction
    Begin,
    /// Insert a document
    Insert,
    /// Update a document
    Update,
    /// Delete a document
    Delete,
    /// Commit a transaction
    Commit,
    /// Rollback a transaction
    Rollback,
}

/// A WAL log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Type of the entry
    pub entry_type:     EntryType,
    /// Transaction ID (UUID)
    pub transaction_id: Uuid,
    /// Collection name
    pub collection:     String,
    /// Document ID
    pub document_id:    String,
    /// Timestamp of the entry
    pub timestamp:      DateTime<Utc>,
    /// Data payload (JSON for insert/update)
    pub data:           Option<serde_json::Value>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(
        entry_type: EntryType,
        transaction_id: Uuid,
        collection: String,
        document_id: String,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            entry_type,
            transaction_id,
            collection,
            document_id,
            timestamp: Utc::now(),
            data,
        }
    }

    /// Serialize the entry to binary format with checksum
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let serialized = bincode::serialize(self)?;
        let mut hasher = Crc32Hasher::new();
        hasher.update(&serialized);
        let checksum = hasher.finalize();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&serialized);
        bytes.extend_from_slice(&checksum.to_le_bytes());

        Ok(bytes)
    }

    /// Deserialize from binary format and verify checksum
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 4 {
            return Err(WalError::InvalidEntry("Entry too short".to_string()));
        }

        let data_len = bytes.len() - 4;
        let data = &bytes[.. data_len];
        let checksum_bytes = &bytes[data_len ..];
        let expected_checksum = u32::from_le_bytes(checksum_bytes.try_into().unwrap());

        let mut hasher = Crc32Hasher::new();
        hasher.update(data);
        let actual_checksum = hasher.finalize();

        if actual_checksum != expected_checksum {
            return Err(WalError::ChecksumMismatch);
        }

        let entry: LogEntry = bincode::deserialize(data)?;
        Ok(entry)
    }
}

/// Write-Ahead Log manager
#[derive(Debug)]
pub struct WalManager {
    /// Path to the WAL file
    path:     PathBuf,
    /// Current WAL file handle
    file:     Arc<tokio::sync::RwLock<File>>,
    /// Current position in the file
    position: Arc<tokio::sync::RwLock<u64>>,
}

impl WalManager {
    /// Create a new WAL manager
    pub async fn new(path: PathBuf) -> Result<Self> {
        info!("Initializing WAL manager at {:?}", path);

        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)
            .await?;

        let position = file.metadata().await?.len();

        Ok(Self {
            path,
            file: Arc::new(tokio::sync::RwLock::new(file)),
            position: Arc::new(tokio::sync::RwLock::new(position)),
        })
    }

    /// Write a log entry to the WAL
    pub async fn write_entry(&self, entry: LogEntry) -> Result<()> {
        debug!("Writing WAL entry: {:?}", entry.entry_type);

        let bytes = entry.to_bytes()?;
        let mut file = self.file.write().await;
        let mut pos = self.position.write().await;

        file.write_all(&bytes).await?;
        file.flush().await?;

        *pos += bytes.len() as u64;

        debug!("WAL entry written successfully");
        Ok(())
    }

    /// Read all entries from the WAL (for recovery)
    pub async fn read_all_entries(&self) -> Result<Vec<LogEntry>> {
        info!("Reading all WAL entries for recovery");

        let mut file = self.file.write().await;
        file.seek(SeekFrom::Start(0)).await?;

        let mut entries = Vec::new();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let mut offset = 0;
        while offset < buffer.len() {
            // Find the next entry by checking checksums
            let mut entry_end = offset;
            while entry_end + 4 <= buffer.len() {
                let _data_len = entry_end - offset;
                let checksum_start = entry_end;
                if checksum_start + 4 > buffer.len() {
                    break;
                }

                let data = &buffer[offset .. entry_end];
                let checksum_bytes = &buffer[checksum_start .. checksum_start + 4];
                let expected_checksum = u32::from_le_bytes(checksum_bytes.try_into().unwrap());

                let mut hasher = Crc32Hasher::new();
                hasher.update(data);
                let actual_checksum = hasher.finalize();

                if actual_checksum == expected_checksum {
                    // Found a valid entry
                    match LogEntry::from_bytes(&buffer[offset .. entry_end + 4]) {
                        Ok(entry) => entries.push(entry),
                        Err(e) => {
                            warn!("Skipping invalid WAL entry: {}", e);
                        },
                    }
                    offset = entry_end + 4;
                    break;
                }
                else {
                    entry_end += 1;
                }
            }

            if entry_end >= buffer.len() {
                break;
            }
        }

        info!("Read {} WAL entries", entries.len());
        Ok(entries)
    }

    /// Perform a checkpoint (truncate the log)
    pub async fn checkpoint(&self) -> Result<()> {
        info!("Performing WAL checkpoint");

        // Close current file
        drop(self.file.write().await);

        // Truncate the file
        tokio::fs::File::create(&self.path).await?;

        // Reopen
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.path)
            .await?;

        *self.file.write().await = file;
        *self.position.write().await = 0;

        info!("WAL checkpoint completed");
        Ok(())
    }

    /// Get the current size of the WAL file
    pub async fn size(&self) -> Result<u64> {
        let metadata = tokio::fs::metadata(&self.path).await?;
        Ok(metadata.len())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn test_log_entry_serialization() {
        let entry = LogEntry::new(
            EntryType::Insert,
            Uuid::new_v4(),
            "users".to_string(),
            "user-123".to_string(),
            Some(json!({"name": "Alice"})),
        );

        let bytes = entry.to_bytes().unwrap();
        let deserialized = LogEntry::from_bytes(&bytes).unwrap();

        assert_eq!(entry.entry_type as u8, deserialized.entry_type as u8);
        assert_eq!(entry.transaction_id, deserialized.transaction_id);
        assert_eq!(entry.collection, deserialized.collection);
        assert_eq!(entry.document_id, deserialized.document_id);
        assert_eq!(entry.data, deserialized.data);
    }

    #[tokio::test]
    async fn test_wal_write_and_read() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        let wal = WalManager::new(wal_path).await.unwrap();

        let entry = LogEntry::new(
            EntryType::Insert,
            Uuid::new_v4(),
            "users".to_string(),
            "user-123".to_string(),
            Some(json!({"name": "Alice"})),
        );

        wal.write_entry(entry.clone()).await.unwrap();

        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].collection, "users");
        assert_eq!(entries[0].document_id, "user-123");
    }

    #[tokio::test]
    async fn test_wal_checkpoint() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        let wal = WalManager::new(wal_path.clone()).await.unwrap();

        let entry = LogEntry::new(
            EntryType::Insert,
            Uuid::new_v4(),
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
