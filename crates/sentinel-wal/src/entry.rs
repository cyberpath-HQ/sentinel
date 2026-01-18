//! WAL log entry types and structures

use chrono::Utc;
use crc32fast::Hasher as Crc32Hasher;
use serde::{Deserialize, Serialize};

use crate::{Result, WalError};

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
    /// Transaction ID (cuid2 string)
    pub transaction_id: String,
    /// Collection name
    pub collection:     String,
    /// Document ID
    pub document_id:    String,
    /// Timestamp of the entry (Unix timestamp in milliseconds)
    pub timestamp:      u64,
    /// Data payload (JSON string for insert/update)
    pub data:           Option<String>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(
        entry_type: EntryType,
        transaction_id: String,
        collection: String,
        document_id: String,
        data: Option<serde_json::Value>,
    ) -> Self {
        let data_str = data.map(|v| v.to_string());
        Self {
            entry_type,
            transaction_id,
            collection,
            document_id,
            timestamp: Utc::now().timestamp_millis() as u64,
            data: data_str,
        }
    }

    /// Serialize the entry to binary format with checksum
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let serialized =
            postcard::to_stdvec(self).map_err(|e: postcard::Error| WalError::Serialization(e.to_string()))?;
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

        let entry: LogEntry =
            postcard::from_bytes(data).map_err(|e: postcard::Error| WalError::Serialization(e.to_string()))?;
        Ok(entry)
    }

    /// Get the data as a JSON Value
    pub fn data_as_value(&self) -> Result<Option<serde_json::Value>> {
        match &self.data {
            Some(s) => {
                let value: serde_json::Value =
                    serde_json::from_str(s).map_err(|e| WalError::Serialization(format!("Invalid JSON: {}", e)))?;
                Ok(Some(value))
            },
            None => Ok(None),
        }
    }
}
