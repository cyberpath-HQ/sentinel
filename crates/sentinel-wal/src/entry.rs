//! WAL log entry types and structures

use chrono::Utc;
use crc32fast::Hasher as Crc32Hasher;
use serde::{Deserialize, Serialize};

use crate::{Result, WalError};

/// Fixed-size byte array for transaction ID (32 bytes)
#[derive(Debug, Clone, PartialEq)]
pub struct FixedBytes32([u8; 32]);

impl Serialize for FixedBytes32 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for FixedBytes32 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: &[u8] = serde::Deserialize::deserialize(deserializer)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes[..32.min(bytes.len())]);
        if bytes.len() < 32 {
            // Pad with zeros if shorter
            arr[bytes.len()..].fill(0);
        }
        Ok(FixedBytes32(arr))
    }
}

impl std::ops::Deref for FixedBytes32 {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for FixedBytes32 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&[u8]> for FixedBytes32 {
    fn from(bytes: &[u8]) -> Self {
        let mut arr = [0u8; 32];
        let len = bytes.len().min(32);
        arr[..len].copy_from_slice(&bytes[..len]);
        FixedBytes32(arr)
    }
}

/// Fixed-size byte array for collection/document ID (256 bytes)
#[derive(Debug, Clone, PartialEq)]
pub struct FixedBytes256([u8; 256]);

impl Serialize for FixedBytes256 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for FixedBytes256 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: &[u8] = serde::Deserialize::deserialize(deserializer)?;
        let mut arr = [0u8; 256];
        let len = bytes.len().min(256);
        arr[..len].copy_from_slice(&bytes[..len]);
        // Pad with zeros if shorter
        arr[len..].fill(0);
        Ok(FixedBytes256(arr))
    }
}

impl std::ops::Deref for FixedBytes256 {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for FixedBytes256 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&[u8]> for FixedBytes256 {
    fn from(bytes: &[u8]) -> Self {
        let mut arr = [0u8; 256];
        let len = bytes.len().min(256);
        arr[..len].copy_from_slice(&bytes[..len]);
        FixedBytes256(arr)
    }
}

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
    /// Transaction ID (32 bytes, fixed length)
    pub transaction_id: FixedBytes32,
    /// Collection name (256 bytes, padded to multiple of 16)
    pub collection:     FixedBytes256,
    /// Document ID (256 bytes, padded to multiple of 16)
    pub document_id:    FixedBytes256,
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
            transaction_id: FixedBytes32::from(transaction_id.as_bytes()),
            collection: FixedBytes256::from(collection.as_bytes()),
            document_id: FixedBytes256::from(document_id.as_bytes()),
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

    /// Get the transaction ID as a string (trimmed)
    pub fn transaction_id_str(&self) -> &str {
        std::str::from_utf8(&self.transaction_id).unwrap().trim_end_matches('\0')
    }

    /// Get the collection name as a string (trimmed)
    pub fn collection_str(&self) -> &str {
        std::str::from_utf8(&self.collection).unwrap().trim_end_matches('\0')
    }

    /// Get the document ID as a string (trimmed)
    pub fn document_id_str(&self) -> &str {
        std::str::from_utf8(&self.document_id).unwrap().trim_end_matches('\0')
    }
}
