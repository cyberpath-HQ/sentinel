//! General metadata structures for collections and stores.
//!
//! This module provides metadata structures that can contain WAL configuration
//! but are not limited to WAL functionality. Metadata includes general collection
//! and store information, statistics, and configuration.

use serde::{Deserialize, Serialize};

use crate::wal::config::CollectionWalConfig;

/// Collection metadata stored on disk.
///
/// This struct contains all persistent metadata for a collection,
/// including WAL configuration, statistics, and operational state.
/// The metadata is designed to be extensible for future features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetadata {
    /// Collection name
    pub name:               String,
    /// Creation timestamp (Unix timestamp)
    pub created_at:         u64,
    /// Last modification timestamp
    pub updated_at:         u64,
    /// WAL configuration for this collection
    pub wal_config:         CollectionWalConfig,
    /// Current WAL checkpoint position (transaction ID)
    pub wal_checkpoint_txn: Option<String>,
    /// Number of documents in the collection
    pub document_count:     u64,
    /// Total size of all documents (bytes)
    pub total_size_bytes:   u64,
    /// Additional metadata fields for future extensibility
    #[serde(flatten)]
    pub extra:              serde_json::Map<String, serde_json::Value>,
}

impl CollectionMetadata {
    /// Create new metadata for a collection
    pub fn new(name: String, wal_config: CollectionWalConfig) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            name: name.clone(),
            created_at: now,
            updated_at: now,
            wal_config,
            wal_checkpoint_txn: None,
            document_count: 0,
            total_size_bytes: 0,
            extra: serde_json::Map::new(),
        }
    }

    /// Update the last modification timestamp
    pub fn touch(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Update document statistics
    pub fn update_stats(&mut self, doc_count: u64, total_size: u64) {
        self.document_count = doc_count;
        self.total_size_bytes = total_size;
        self.touch();
    }
}

/// Store metadata stored on disk.
///
/// This struct contains all persistent metadata for a store,
/// including global configuration and operational state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMetadata {
    /// Creation timestamp (Unix timestamp)
    pub created_at:       u64,
    /// Last modification timestamp
    pub updated_at:       u64,
    /// WAL configuration for the store
    pub wal_config:       crate::wal::config::StoreWalConfig,
    /// Total number of collections in the store
    pub collection_count: u64,
    /// Total size of all collections (bytes)
    pub total_size_bytes: u64,
    /// Additional metadata fields for future extensibility
    #[serde(flatten)]
    pub extra:            serde_json::Map<String, serde_json::Value>,
}

impl StoreMetadata {
    /// Create new metadata for a store
    pub fn new(wal_config: crate::wal::config::StoreWalConfig) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            created_at: now,
            updated_at: now,
            wal_config,
            collection_count: 0,
            total_size_bytes: 0,
            extra: serde_json::Map::new(),
        }
    }

    /// Update the last modification timestamp
    pub fn touch(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Update collection statistics
    pub fn update_stats(&mut self, collection_count: u64, total_size: u64) {
        self.collection_count = collection_count;
        self.total_size_bytes = total_size;
        self.touch();
    }
}
