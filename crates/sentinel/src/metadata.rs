//! General metadata structures for collections and stores.
//!
//! This module provides metadata structures that are DBMS-wide.
//! Metadata includes general collection and store information, statistics, and configuration with
//! proper versioning.
//!
//! ## Storage Limits
//!
//! Collection metadata files are limited to 1MB total size to prevent unbounded growth.
//! Store metadata files are limited to 10MB total size.
//! These limits ensure metadata operations remain performant and prevent abuse.

use serde::{Deserialize, Serialize};
use sentinel_wal::{CollectionWalConfig, StoreWalConfig};

use crate::META_SENTINEL_VERSION;

/// Version of the metadata format.
///
/// This is a numeric version that supports fast-forward migration.
/// Higher versions can read and migrate older metadata formats.
pub type MetadataVersion = u32;

/// Collection metadata stored on disk.
///
/// This struct contains all persistent metadata for a collection,
/// including statistics, operational state, and WAL configuration.
///
/// Storage limit: 1MB total serialized size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetadata {
    /// Metadata format version
    pub version:          MetadataVersion,
    /// Collection name
    pub name:             String,
    /// Creation timestamp (Unix timestamp)
    pub created_at:       u64,
    /// Last modification timestamp
    pub updated_at:       u64,
    /// Number of documents in the collection
    pub document_count:   u64,
    /// Total size of all documents (bytes)
    pub total_size_bytes: u64,
    /// WAL configuration for this collection
    pub wal_config:       Option<CollectionWalConfig>,
}

impl CollectionMetadata {
    /// Create new metadata for a collection
    pub fn new(name: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version:          META_SENTINEL_VERSION,
            name:             name.clone(),
            created_at:       now,
            updated_at:       now,
            document_count:   0,
            total_size_bytes: 0,
            wal_config:       None,
        }
    }

    /// Upgrade metadata to the current version if needed
    ///
    /// This method handles forward migration of metadata from older versions
    /// to the current version. It modifies the metadata in-place.
    pub fn upgrade_to_current(&mut self) -> Result<(), String> {
        let current_version = META_SENTINEL_VERSION;

        while self.version < current_version {
            match self.version {
                1 => {
                    // Version 1 -> 2: Add any new fields with defaults
                    // Currently no changes needed for version 2

                    // this is currently a no-op, but we set the version to 2
                    // when we add new fields in future versions
                    self.version = 2;
                },
                // Add future version migrations here as needed
                // 2 => { /* migration logic */ self.version = 3; }
                _ => {
                    return Err(format!(
                        "Unsupported metadata version: {} (current: {})",
                        self.version, current_version
                    ));
                },
            }
        }

        Ok(())
    }

    /// Check if metadata needs upgrading
    pub fn needs_upgrade(&self) -> bool { self.version < META_SENTINEL_VERSION }

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
/// including global statistics, operational state, and WAL configuration.
///
/// Storage limit: 10MB total serialized size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMetadata {
    /// Metadata format version
    pub version:          MetadataVersion,
    /// Store creation timestamp
    pub created_at:       u64,
    /// Last modification timestamp
    pub updated_at:       u64,
    /// Total number of collections
    pub collection_count: u64,
    /// Total number of documents across all collections
    pub total_documents:  u64,
    /// Total size of all data (bytes)
    pub total_size_bytes: u64,
    /// WAL configuration for the store
    pub wal_config:       StoreWalConfig,
}

impl StoreMetadata {
    /// Create new metadata for a store
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version:          META_SENTINEL_VERSION,
            created_at:       now,
            updated_at:       now,
            collection_count: 0,
            total_documents:  0,
            total_size_bytes: 0,
            wal_config:       StoreWalConfig::default(),
        }
    }
}

impl Default for StoreMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl StoreMetadata {
    /// Upgrade metadata to the current version if needed
    ///
    /// This method handles forward migration of metadata from older versions
    /// to the current version. It modifies the metadata in-place.
    pub fn upgrade_to_current(&mut self) -> Result<(), String> {
        let current_version = META_SENTINEL_VERSION;

        while self.version < current_version {
            match self.version {
                1 => {
                    // Version 1 -> 2: Add any new fields with defaults
                    // Currently no changes needed for version 2

                    // this is currently a no-op, but we set the version to 2
                    // when we add new fields in future versions
                    self.version = 2;
                },
                // Add future version migrations here as needed
                // 2 => { /* migration logic */ self.version = 3; }
                _ => {
                    return Err(format!(
                        "Unsupported metadata version: {} (current: {})",
                        self.version, current_version
                    ));
                },
            }
        }

        Ok(())
    }

    /// Check if metadata needs upgrading
    pub fn needs_upgrade(&self) -> bool { self.version < META_SENTINEL_VERSION }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_metadata_new() {
        let metadata = CollectionMetadata::new("test_collection".to_string());
        assert_eq!(metadata.version, META_SENTINEL_VERSION);
        assert_eq!(metadata.name, "test_collection");
        assert_eq!(metadata.document_count, 0);
        assert_eq!(metadata.total_size_bytes, 0);
        assert!(
            metadata.created_at <=
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
        );
        assert_eq!(metadata.created_at, metadata.updated_at);
    }

    #[test]
    fn test_collection_metadata_add_remove_document() {
        let mut metadata = CollectionMetadata::new("test".to_string());

        // Add document
        metadata.add_document(100);
        assert_eq!(metadata.document_count, 1);
        assert_eq!(metadata.total_size_bytes, 100);
        assert!(metadata.updated_at >= metadata.created_at);

        let updated_at = metadata.updated_at;

        // Add another document
        metadata.add_document(200);
        assert_eq!(metadata.document_count, 2);
        assert_eq!(metadata.total_size_bytes, 300);
        assert!(metadata.updated_at >= updated_at);

        // Remove document
        metadata.remove_document(100);
        assert_eq!(metadata.document_count, 1);
        assert_eq!(metadata.total_size_bytes, 200);

        // Remove last document
        metadata.remove_document(200);
        assert_eq!(metadata.document_count, 0);
        assert_eq!(metadata.total_size_bytes, 0);
    }

    #[test]
    fn test_collection_metadata_update_document_size() {
        let mut metadata = CollectionMetadata::new("test".to_string());
        metadata.add_document(100);

        metadata.update_document_size(100, 150);
        assert_eq!(metadata.document_count, 1);
        assert_eq!(metadata.total_size_bytes, 150);
    }

    #[test]
    fn test_collection_metadata_upgrade() {
        let mut metadata = CollectionMetadata::new("test".to_string());
        metadata.version = 1;

        assert!(metadata.needs_upgrade());
        assert!(metadata.upgrade_to_current().is_ok());
    }

    #[test]
    fn test_store_metadata_new() {
        let metadata = StoreMetadata::new();
        assert_eq!(metadata.version, META_SENTINEL_VERSION);
        assert_eq!(metadata.collection_count, 0);
        assert_eq!(metadata.total_documents, 0);
        assert_eq!(metadata.total_size_bytes, 0);
        assert!(
            metadata.created_at <=
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
        );
    }

    #[test]
    fn test_store_metadata_operations() {
        let mut metadata = StoreMetadata::new();

        // Add collection
        metadata.add_collection();
        assert_eq!(metadata.collection_count, 1);

        // Update documents
        metadata.update_documents(5, 1000);
        assert_eq!(metadata.total_documents, 5);
        assert_eq!(metadata.total_size_bytes, 1000);

        // Update again
        metadata.update_documents(3, 500);
        assert_eq!(metadata.total_documents, 8);
        assert_eq!(metadata.total_size_bytes, 1500);

        // Negative update (remove documents)
        metadata.update_documents(-2, -200);
        assert_eq!(metadata.total_documents, 6);
        assert_eq!(metadata.total_size_bytes, 1300);

        // Remove collection
        metadata.remove_collection();
        assert_eq!(metadata.collection_count, 0);
    }

    #[test]
    fn test_store_metadata_upgrade() {
        let mut metadata = StoreMetadata::new();
        metadata.version = 1;

        assert!(metadata.needs_upgrade());
        assert!(metadata.upgrade_to_current().is_ok());
    }

    #[test]
    fn test_metadata_serialization() {
        let collection_meta = CollectionMetadata::new("test".to_string());
        let serialized = serde_json::to_string(&collection_meta).unwrap();
        let deserialized: CollectionMetadata = serde_json::from_str(&serialized).unwrap();
        assert_eq!(collection_meta.name, deserialized.name);
        assert_eq!(collection_meta.version, deserialized.version);

        let store_meta = StoreMetadata::new();
        let serialized = serde_json::to_string(&store_meta).unwrap();
        let deserialized: StoreMetadata = serde_json::from_str(&serialized).unwrap();
        assert_eq!(store_meta.version, deserialized.version);
    }
}
