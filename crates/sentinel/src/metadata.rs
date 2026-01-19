//! General metadata structures for collections and stores.
//!
//! This module provides metadata structures that are DBMS-wide. 
//! Metadata includes general collection and store information, statistics, and configuration with proper versioning.

use serde::{Deserialize, Serialize};

/// Version of the metadata format.
///
/// This allows for forward-compatible metadata evolution.
/// Newer versions can read older metadata and migrate forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetadataVersion {
    /// Initial metadata format (v1.0)
    V1,
}

impl Default for MetadataVersion {
    fn default() -> Self {
        MetadataVersion::V1
    }
}

/// Collection metadata stored on disk.
///
/// This struct contains all persistent metadata for a collection,
/// including statistics and operational state. WAL configuration
/// is handled separately to avoid tight coupling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetadata {
    /// Metadata format version
    pub version:         MetadataVersion,
    /// Collection name
    pub name:            String,
    /// Creation timestamp (Unix timestamp)
    pub created_at:      u64,
    /// Last modification timestamp
    pub updated_at:      u64,
    /// Number of documents in the collection
    pub document_count:  u64,
    /// Total size of all documents (bytes)
    pub total_size_bytes: u64,
}

impl CollectionMetadata {
    /// Create new metadata for a collection
    pub fn new(name: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version:          MetadataVersion::V1,
            name:             name.clone(),
            created_at:       now,
            updated_at:       now,
            document_count:   0,
            total_size_bytes: 0,
        }
    }

    /// Update the modification timestamp
    pub fn touch(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Increment document count and size
    pub fn add_document(&mut self, size_bytes: u64) {
        self.document_count += 1;
        self.total_size_bytes += size_bytes;
        self.touch();
    }

    /// Decrement document count and size
    pub fn remove_document(&mut self, size_bytes: u64) {
        self.document_count = self.document_count.saturating_sub(1);
        self.total_size_bytes = self.total_size_bytes.saturating_sub(size_bytes);
        self.touch();
    }

    /// Update document size (for modifications)
    pub fn update_document_size(&mut self, old_size: u64, new_size: u64) {
        self.total_size_bytes = self.total_size_bytes.saturating_sub(old_size) + new_size;
        self.touch();
    }
}

/// Store metadata stored on disk.
///
/// This struct contains all persistent metadata for the store,
/// including global statistics and operational state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMetadata {
    /// Metadata format version
    pub version:         MetadataVersion,
    /// Store creation timestamp
    pub created_at:      u64,
    /// Last modification timestamp
    pub updated_at:      u64,
    /// Total number of collections
    pub collection_count: u64,
    /// Total number of documents across all collections
    pub total_documents: u64,
    /// Total size of all data (bytes)
    pub total_size_bytes: u64,
}

impl StoreMetadata {
    /// Create new metadata for a store
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version:           MetadataVersion::V1,
            created_at:        now,
            updated_at:        now,
            collection_count:  0,
            total_documents:   0,
            total_size_bytes:  0,
        }
    }

    /// Update the modification timestamp
    pub fn touch(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Add a collection
    pub fn add_collection(&mut self) {
        self.collection_count += 1;
        self.touch();
    }

    /// Remove a collection
    pub fn remove_collection(&mut self) {
        self.collection_count = self.collection_count.saturating_sub(1);
        self.touch();
    }

    /// Update document statistics
    pub fn update_documents(&mut self, document_delta: i64, size_delta: i64) {
        self.total_documents = (self.total_documents as i64 + document_delta).max(0) as u64;
        self.total_size_bytes = (self.total_size_bytes as i64 + size_delta).max(0) as u64;
        self.touch();
    }
}