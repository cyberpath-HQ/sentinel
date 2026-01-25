//! WAL log entry types and structures

use chrono::Utc;
use crc32fast::Hasher as Crc32Hasher;
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::{Result, WalError};

/// Fixed-size byte array for transaction ID (32 bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
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
        arr.copy_from_slice(&bytes[.. 32.min(bytes.len())]);
        if bytes.len() < 32 {
            // Pad with zeros if shorter
            arr[bytes.len() ..].fill(0);
        }
        Ok(FixedBytes32(arr))
    }
}

impl std::ops::Deref for FixedBytes32 {
    type Target = [u8];

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for FixedBytes32 {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl From<&[u8]> for FixedBytes32 {
    fn from(bytes: &[u8]) -> Self {
        let mut temp = bytes.to_vec();
        let len = temp.len();
        let padded_len = ((len + 15) / 16) * 16;
        temp.resize(padded_len, 0);
        let mut arr = [0u8; 32];
        let copy_len = temp.len().min(32);
        arr[.. copy_len].copy_from_slice(&temp[.. copy_len]);
        FixedBytes32(arr)
    }
}

/// Fixed-size byte array for collection/document ID (padded to multiple of 16, max 256)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedBytes256(Vec<u8>);

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
        Ok(FixedBytes256(bytes.to_vec()))
    }
}

impl std::ops::Deref for FixedBytes256 {
    type Target = [u8];

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for FixedBytes256 {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl From<&[u8]> for FixedBytes256 {
    fn from(bytes: &[u8]) -> Self {
        let mut temp = bytes.to_vec();
        let len = temp.len();
        let padded_len = ((len + 15) / 16) * 16;
        if padded_len > 256 {
            temp.truncate(256);
            // If truncated, pad to 256
            temp.resize(256, 0);
        }
        else {
            temp.resize(padded_len, 0);
        }
        FixedBytes256(temp)
    }
}

/// Types of WAL entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Create a new log entry with the specified parameters.
    ///
    /// This constructor generates a unique transaction ID using CUID2 and captures
    /// the current timestamp. The data is serialized to JSON string format if provided.
    ///
    /// # Arguments
    ///
    /// * `entry_type` - The type of operation (Insert, Update, Delete)
    /// * `collection` - Name of the collection this entry belongs to
    /// * `document_id` - Unique identifier of the document
    /// * `data` - Optional JSON data payload (for insert/update operations)
    ///
    /// # Returns
    ///
    /// Returns a new `LogEntry` instance with a generated transaction ID and timestamp.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_wal::{LogEntry, EntryType};
    /// use serde_json::json;
    ///
    /// // Create an insert entry
    /// let entry = LogEntry::new(
    ///     EntryType::Insert,
    ///     "users".to_string(),
    ///     "user-123".to_string(),
    ///     Some(json!({"name": "Alice", "age": 30}))
    /// );
    ///
    /// // Create a delete entry (no data needed)
    /// let delete_entry = LogEntry::new(
    ///     EntryType::Delete,
    ///     "users".to_string(),
    ///     "user-123".to_string(),
    ///     None
    /// );
    /// ```
    pub fn new(
        entry_type: EntryType,
        collection: String,
        document_id: String,
        data: Option<serde_json::Value>,
    ) -> Self {
        let transaction_id = cuid2::CuidConstructor::new().with_length(32).create_id();
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

    /// Serialize the entry to binary format with checksum.
    ///
    /// This method serializes the log entry using Postcard (a compact binary format)
    /// and appends a CRC32 checksum for data integrity verification. The binary format
    /// is used for efficient storage and fast I/O operations.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the serialized bytes with checksum, or a `WalError`
    /// if serialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_wal::{EntryType, LogEntry};
    ///
    /// let entry = LogEntry::new(
    ///     EntryType::Insert,
    ///     "users".to_string(),
    ///     "user-123".to_string(),
    ///     None,
    /// );
    ///
    /// let bytes = entry.to_bytes().unwrap();
    /// assert!(!bytes.is_empty());
    /// // The serialized data includes the entry plus a 4-byte CRC32 checksum
    /// assert!(bytes.len() > 4);
    /// ```
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let serialized =
            postcard::to_stdvec(self).map_err(|e: postcard::Error| WalError::Serialization(e.to_string()))?;
        let mut hasher = Crc32Hasher::new();
        hasher.update(&serialized);
        let checksum = hasher.finalize();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&serialized);
        bytes.extend_from_slice(&checksum.to_le_bytes());

        trace!(
            "Serialized entry to {} bytes (entry_type: {:?})",
            bytes.len(),
            self.entry_type
        );
        Ok(bytes)
    }

    /// Deserialize from binary format and verify checksum.
    ///
    /// This method deserializes a log entry from Postcard binary format and verifies
    /// the CRC32 checksum to ensure data integrity. The last 4 bytes of the input
    /// are expected to contain the checksum.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The binary data containing the serialized entry and checksum
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the deserialized `LogEntry`, or a `WalError`
    /// if deserialization fails or checksum verification fails.
    ///
    /// # Errors
    ///
    /// * `WalError::InvalidEntry` - If the data is too short (less than 4 bytes for checksum)
    /// * `WalError::ChecksumMismatch` - If the calculated checksum doesn't match the stored one
    /// * `WalError::Serialization` - If Postcard deserialization fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_wal::{EntryType, LogEntry};
    ///
    /// let entry = LogEntry::new(
    ///     EntryType::Insert,
    ///     "users".to_string(),
    ///     "user-123".to_string(),
    ///     None,
    /// );
    ///
    /// let bytes = entry.to_bytes().unwrap();
    /// let deserialized = LogEntry::from_bytes(&bytes).unwrap();
    ///
    /// assert_eq!(deserialized.entry_type, EntryType::Insert);
    /// assert_eq!(deserialized.collection_str(), "users");
    /// assert_eq!(deserialized.document_id_str(), "user-123");
    /// ```
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
        trace!(
            "Deserialized binary entry (entry_type: {:?})",
            entry.entry_type
        );
        Ok(entry)
    }

    /// Serialize the entry to JSON format.
    ///
    /// This method converts the log entry to a human-readable JSON Lines format.
    /// All fields are included in the JSON output, with string representations
    /// for binary fields (transaction_id, collection, document_id).
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the JSON string representation, or a `WalError`
    /// if JSON serialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_wal::{LogEntry, EntryType};
    /// use serde_json::json;
    ///
    /// let entry = LogEntry::new(
    ///     EntryType::Insert,
    ///     "users".to_string(),
    ///     "user-123".to_string(),
    ///     Some(json!({"name": "Alice"}))
    /// );
    ///
    /// let json_str = entry.to_json().unwrap();
    /// println!("{}", json_str);
    /// // Output: {"entry_type":"Insert","transaction_id":"...","collection":"users","document_id":"user-123","timestamp":1234567890,"data":"{\"name\":\"Alice\"}"}
    /// ```
    pub fn to_json(&self) -> Result<String> {
        let json_value = serde_json::json!({
            "entry_type": self.entry_type,
            "transaction_id": self.transaction_id_str(),
            "collection": self.collection_str(),
            "document_id": self.document_id_str(),
            "timestamp": self.timestamp,
            "data": self.data
        });
        let json_str = serde_json::to_string(&json_value)
            .map_err(|e| WalError::Serialization(format!("JSON serialization error: {}", e)))?;
        trace!(
            "Serialized entry to JSON (entry_type: {:?})",
            self.entry_type
        );
        Ok(json_str)
    }

    /// Deserialize from JSON format.
    ///
    /// This method parses a log entry from JSON Lines format. All required fields
    /// must be present in the JSON object. String fields are converted back to
    /// their fixed-size binary representations.
    ///
    /// # Arguments
    ///
    /// * `json_str` - The JSON string representation of a log entry
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the deserialized `LogEntry`, or a `WalError`
    /// if parsing fails or required fields are missing.
    ///
    /// # Errors
    ///
    /// * `WalError::InvalidEntry` - If required fields are missing or have wrong types
    /// * `WalError::Serialization` - If JSON parsing fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_wal::{EntryType, LogEntry};
    ///
    /// let json_str = r#"{
    ///     "entry_type": "Insert",
    ///     "transaction_id": "abc123",
    ///     "collection": "users",
    ///     "document_id": "user-123",
    ///     "timestamp": 1234567890,
    ///     "data": "{\"name\":\"Alice\"}"
    /// }"#;
    ///
    /// let entry = LogEntry::from_json(json_str).unwrap();
    /// assert_eq!(entry.entry_type, EntryType::Insert);
    /// assert_eq!(entry.collection_str(), "users");
    /// ```
    pub fn from_json(json_str: &str) -> Result<Self> {
        let json_value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| WalError::Serialization(format!("JSON parsing error: {}", e)))?;

        let entry_type = match json_value.get("entry_type") {
            Some(v) => {
                serde_json::from_value(v.clone())
                    .map_err(|e| WalError::Serialization(format!("Invalid entry_type: {}", e)))?
            },
            None => return Err(WalError::InvalidEntry("Missing entry_type".to_string())),
        };

        let transaction_id = match json_value.get("transaction_id") {
            Some(v) => {
                v.as_str()
                    .ok_or_else(|| WalError::InvalidEntry("transaction_id must be string".to_string()))?
            },
            None => return Err(WalError::InvalidEntry("Missing transaction_id".to_string())),
        };

        let collection = match json_value.get("collection") {
            Some(v) => {
                v.as_str()
                    .ok_or_else(|| WalError::InvalidEntry("collection must be string".to_string()))?
            },
            None => return Err(WalError::InvalidEntry("Missing collection".to_string())),
        };

        let document_id = match json_value.get("document_id") {
            Some(v) => {
                v.as_str()
                    .ok_or_else(|| WalError::InvalidEntry("document_id must be string".to_string()))?
            },
            None => return Err(WalError::InvalidEntry("Missing document_id".to_string())),
        };

        let timestamp = match json_value.get("timestamp") {
            Some(v) => {
                v.as_u64()
                    .ok_or_else(|| WalError::InvalidEntry("timestamp must be number".to_string()))?
            },
            None => return Err(WalError::InvalidEntry("Missing timestamp".to_string())),
        };

        let data = json_value
            .get("data")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let entry = LogEntry {
            entry_type,
            transaction_id: FixedBytes32::from(transaction_id.as_bytes()),
            collection: FixedBytes256::from(collection.as_bytes()),
            document_id: FixedBytes256::from(document_id.as_bytes()),
            timestamp,
            data,
        };
        trace!(
            "Deserialized JSON entry (entry_type: {:?})",
            entry.entry_type
        );
        Ok(entry)
    }

    /// Get the data as a JSON Value.
    ///
    /// This method parses the stored JSON string data into a `serde_json::Value`
    /// for programmatic access to the document data.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing `Some(Value)` if data exists and is valid JSON,
    /// `None` if no data is stored, or a `WalError` if JSON parsing fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_wal::{LogEntry, EntryType};
    /// use serde_json::json;
    ///
    /// let entry = LogEntry::new(
    ///     EntryType::Insert,
    ///     "users".to_string(),
    ///     "user-123".to_string(),
    ///     Some(json!({"name": "Alice", "age": 30}))
    /// );
    ///
    /// let data = entry.data_as_value().unwrap().unwrap();
    /// assert_eq!(data["name"], "Alice");
    /// assert_eq!(data["age"], 30);
    /// ```
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

    /// Get the transaction ID as a string (trimmed).
    ///
    /// Returns the transaction ID with null bytes trimmed from the end.
    /// Transaction IDs are generated using CUID2 and are guaranteed to be valid UTF-8.
    ///
    /// # Returns
    ///
    /// Returns the transaction ID as a string slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_wal::{EntryType, LogEntry};
    ///
    /// let entry = LogEntry::new(
    ///     EntryType::Insert,
    ///     "users".to_string(),
    ///     "user-123".to_string(),
    ///     None,
    /// );
    ///
    /// let tx_id = entry.transaction_id_str();
    /// assert!(!tx_id.is_empty());
    /// // Transaction IDs are unique identifiers generated using CUID2
    /// println!("Transaction ID: {}", tx_id);
    /// ```
    pub fn transaction_id_str(&self) -> &str {
        std::str::from_utf8(&self.transaction_id)
            .unwrap()
            .trim_end_matches('\0')
    }

    /// Get the collection name as a string (trimmed).
    ///
    /// Returns the collection name with null bytes trimmed from the end.
    /// Collection names are stored as UTF-8 strings.
    ///
    /// # Returns
    ///
    /// Returns the collection name as a string slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_wal::{EntryType, LogEntry};
    ///
    /// let entry = LogEntry::new(
    ///     EntryType::Insert,
    ///     "users".to_string(),
    ///     "user-123".to_string(),
    ///     None,
    /// );
    ///
    /// assert_eq!(entry.collection_str(), "users");
    /// ```
    pub fn collection_str(&self) -> &str {
        std::str::from_utf8(&self.collection)
            .unwrap()
            .trim_end_matches('\0')
    }

    /// Get the document ID as a string (trimmed).
    ///
    /// Returns the document ID with null bytes trimmed from the end.
    /// Document IDs are stored as UTF-8 strings.
    ///
    /// # Returns
    ///
    /// Returns the document ID as a string slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_wal::{EntryType, LogEntry};
    ///
    /// let entry = LogEntry::new(
    ///     EntryType::Insert,
    ///     "users".to_string(),
    ///     "user-123".to_string(),
    ///     None,
    /// );
    ///
    /// assert_eq!(entry.document_id_str(), "user-123");
    /// ```
    pub fn document_id_str(&self) -> &str {
        std::str::from_utf8(&self.document_id)
            .unwrap()
            .trim_end_matches('\0')
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    // Tests for FixedBytes32
    #[test]
    fn test_fixed_bytes32_from_slice() {
        let input = b"hello world" as &[u8];
        let fixed = FixedBytes32::from(input);
        assert_eq!(&fixed[.. 11], input);
    }

    #[test]
    fn test_fixed_bytes32_from_slice_longer_than_32() {
        let input = b"this is a very long string that exceeds 32 bytes in length" as &[u8];
        let fixed = FixedBytes32::from(input);
        assert_eq!(fixed.len(), 32);
        assert_eq!(&fixed[.. 32], &input[.. 32]);
    }

    #[test]
    fn test_fixed_bytes32_serialization() {
        let input = b"test data" as &[u8];
        let fixed = FixedBytes32::from(input);

        let serialized = serde_json::to_string(&fixed).unwrap();
        // Just verify it serializes without error
        assert!(!serialized.is_empty());
    }

    #[test]
    fn test_fixed_bytes32_equality() {
        let bytes1 = FixedBytes32::from(b"same" as &[u8]);
        let bytes2 = FixedBytes32::from(b"same" as &[u8]);
        assert_eq!(bytes1, bytes2);

        let bytes3 = FixedBytes32::from(b"different" as &[u8]);
        assert_ne!(bytes1, bytes3);
    }

    #[test]
    fn test_fixed_bytes32_clone() {
        let original = FixedBytes32::from(b"test" as &[u8]);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_fixed_bytes32_deref() {
        let fixed = FixedBytes32::from(b"hello" as &[u8]);
        let slice: &[u8] = &*fixed;
        assert_eq!(&slice[.. 5], b"hello");
    }

    #[test]
    fn test_fixed_bytes32_deref_mut() {
        let mut fixed = FixedBytes32::from(b"hello" as &[u8]);
        fixed[0] = b'H';
        assert_eq!(fixed[0], b'H');
    }

    // Tests for FixedBytes256
    #[test]
    fn test_fixed_bytes256_from_slice() {
        let input = b"collection name" as &[u8];
        let fixed = FixedBytes256::from(input);
        assert_eq!(&fixed[.. 15], input);
    }

    #[test]
    fn test_fixed_bytes256_serialization() {
        let input = b"test_collection" as &[u8];
        let fixed = FixedBytes256::from(input);

        let serialized = serde_json::to_string(&fixed).unwrap();
        // Just verify it serializes without error
        assert!(!serialized.is_empty());
    }

    #[test]
    fn test_fixed_bytes256_equality() {
        let bytes1 = FixedBytes256::from(b"collection" as &[u8]);
        let bytes2 = FixedBytes256::from(b"collection" as &[u8]);
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn test_fixed_bytes256_clone() {
        let original = FixedBytes256::from(b"document-id" as &[u8]);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_fixed_bytes256_padding() {
        let input = b"test" as &[u8];
        let fixed = FixedBytes256::from(input);
        // Should be padded to multiple of 16
        assert_eq!(fixed.len() % 16, 0);
    }

    // Tests for EntryType
    #[test]
    fn test_entry_type_equality() {
        assert_eq!(EntryType::Insert, EntryType::Insert);
        assert_ne!(EntryType::Insert, EntryType::Delete);
    }

    #[test]
    fn test_entry_type_clone() {
        let original = EntryType::Update;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_entry_type_serialization() {
        let entry_types = vec![
            EntryType::Begin,
            EntryType::Insert,
            EntryType::Update,
            EntryType::Delete,
            EntryType::Commit,
            EntryType::Rollback,
        ];

        for entry_type in entry_types {
            let serialized = serde_json::to_string(&entry_type).unwrap();
            let deserialized: EntryType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(entry_type, deserialized);
        }
    }

    #[test]
    fn test_entry_type_debug() {
        let debug_str = format!("{:?}", EntryType::Insert);
        assert!(debug_str.contains("Insert"));
    }

    // Tests for LogEntry
    #[test]
    fn test_log_entry_new_with_data() {
        let entry = LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-123".to_string(),
            Some(json!({"name": "Alice"})),
        );

        assert_eq!(entry.entry_type, EntryType::Insert);
        assert!(entry.data.is_some());
        assert!(entry.timestamp > 0);
    }

    #[test]
    fn test_log_entry_new_without_data() {
        let entry = LogEntry::new(
            EntryType::Delete,
            "users".to_string(),
            "user-123".to_string(),
            None,
        );

        assert_eq!(entry.entry_type, EntryType::Delete);
        assert!(entry.data.is_none());
    }

    #[test]
    fn test_log_entry_to_bytes() {
        let entry = LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-123".to_string(),
            Some(json!({"name": "Bob"})),
        );

        let bytes = entry.to_bytes().unwrap();
        assert!(!bytes.is_empty());
        assert!(bytes.len() > 4); // Should include CRC32 checksum
    }

    #[test]
    fn test_log_entry_from_bytes_roundtrip() {
        let original = LogEntry::new(
            EntryType::Update,
            "orders".to_string(),
            "order-456".to_string(),
            Some(json!({"status": "shipped", "cost": 99.99})),
        );

        let bytes = original.to_bytes().unwrap();
        let restored = LogEntry::from_bytes(&bytes).unwrap();

        assert_eq!(original.entry_type, restored.entry_type);
        assert_eq!(original.data, restored.data);
    }

    #[test]
    fn test_log_entry_from_bytes_invalid_checksum() {
        let entry = LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            None,
        );

        let mut bytes = entry.to_bytes().unwrap();
        // Corrupt the checksum
        let last_idx = bytes.len() - 1;
        bytes[last_idx] ^= 0xff;

        let result = LogEntry::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_log_entry_from_bytes_truncated() {
        let result = LogEntry::from_bytes(b"truncated");
        assert!(result.is_err());
    }

    #[test]
    fn test_log_entry_collection_str() {
        let entry = LogEntry::new(
            EntryType::Insert,
            "my_collection".to_string(),
            "doc-1".to_string(),
            None,
        );

        assert_eq!(entry.collection_str(), "my_collection");
    }

    #[test]
    fn test_log_entry_document_id_str() {
        let entry = LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-abc-123".to_string(),
            None,
        );

        assert_eq!(entry.document_id_str(), "user-abc-123");
    }

    #[test]
    fn test_log_entry_collection_str_with_nulls() {
        let entry = LogEntry::new(
            EntryType::Insert,
            "collection".to_string(),
            "doc".to_string(),
            None,
        );
        // collection is padded with zeros
        let collection_str = entry.collection_str();
        assert_eq!(collection_str, "collection");
    }

    #[test]
    fn test_log_entry_document_id_str_with_nulls() {
        let entry = LogEntry::new(
            EntryType::Delete,
            "col".to_string(),
            "document-id".to_string(),
            None,
        );
        let document_id_str = entry.document_id_str();
        assert_eq!(document_id_str, "document-id");
    }

    #[test]
    fn test_log_entry_clone() {
        let original = LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(json!({"name": "Test"})),
        );

        let cloned = original.clone();
        assert_eq!(original.entry_type, cloned.entry_type);
        assert_eq!(original.data, cloned.data);
    }

    #[test]
    fn test_log_entry_equality() {
        let entry1 = LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(json!({"name": "Alice"})),
        );

        let entry2 = entry1.clone();
        assert_eq!(entry1, entry2);
    }

    #[test]
    fn test_log_entry_various_entry_types() {
        let entry_types = vec![
            (EntryType::Begin, "test_col", "txn-1", None),
            (EntryType::Insert, "users", "user-1", Some(json!({"id": 1}))),
            (EntryType::Update, "users", "user-2", Some(json!({"id": 2}))),
            (EntryType::Delete, "users", "user-3", None),
            (EntryType::Commit, "test_col", "txn-2", None),
            (EntryType::Rollback, "test_col", "txn-3", None),
        ];

        for (entry_type, col, doc, data) in entry_types {
            let entry = LogEntry::new(entry_type, col.to_string(), doc.to_string(), data);
            assert_eq!(entry.entry_type, entry_type);
            assert_eq!(entry.collection_str(), col);
            assert_eq!(entry.document_id_str(), doc);
        }
    }

    #[test]
    fn test_log_entry_postcard_roundtrip_with_json() {
        // Test postcard serialization roundtrip which preserves FixedBytes types correctly
        let entry = LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(json!({"name": "Dave", "age": 25})),
        );

        let bytes = postcard::to_stdvec(&entry).unwrap();
        let restored: LogEntry = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(entry.entry_type, restored.entry_type);
        assert_eq!(entry.data, restored.data);
        assert_eq!(entry.collection_str(), restored.collection_str());
        assert_eq!(entry.document_id_str(), restored.document_id_str());
    }
}
