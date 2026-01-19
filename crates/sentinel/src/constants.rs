//! Constants for special file and directory names used throughout Sentinel.
//!
//! This module centralizes all special names to prevent typos and ensure consistency.
//! All names are documented with their purpose and usage patterns.

/// Directory name for storing collection data within a store.
pub const DATA_DIR: &str = "data";

/// File extension for document files.
pub const DOCUMENT_EXTENSION: &str = "json";

/// Directory name for Write-Ahead Log files within a collection.
pub const WAL_DIR: &str = ".wal";

/// Filename for the main WAL file within a collection's WAL directory.
pub const WAL_FILE: &str = "transactions.wal";

/// Directory name for soft-deleted documents within a collection.
pub const DELETED_DIR: &str = ".deleted";

/// Filename for collection metadata stored within a collection directory.
pub const COLLECTION_METADATA_FILE: &str = ".metadata.json";

/// Filename for store metadata stored in the store root directory.
pub const STORE_METADATA_FILE: &str = ".store.json";

/// Name of the special collection used for storing encryption keys.
pub const KEYS_COLLECTION: &str = ".keys";

/// Filename for storing signing keys within the keys collection.
pub const SIGNING_KEY_FILE: &str = "signing_key";

/// Maximum size for collection metadata files (1MB).
pub const MAX_COLLECTION_METADATA_SIZE: u64 = 1024 * 1024;

/// Maximum size for store metadata files (10MB).
pub const MAX_STORE_METADATA_SIZE: u64 = 10 * 1024 * 1024;
