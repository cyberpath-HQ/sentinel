//! WAL configuration structures and operational modes.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Failure handling modes for WAL operations.
///
/// These modes control how WAL-related failures are handled:
/// - `Disabled`: WAL operations are skipped entirely
/// - `Warn`: WAL failures are logged as warnings but don't fail operations
/// - `Strict`: WAL failures cause operations to fail immediately
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WalFailureMode {
    /// WAL operations are completely disabled
    Disabled,
    /// WAL failures are logged as warnings but operations continue
    Warn,
    /// WAL failures cause operations to fail (default for data integrity)
    #[default]
    Strict,
}

/// Configuration for WAL operations at the collection level.
///
/// This struct defines how WAL should behave for a specific collection,
/// including operational modes, verification settings, recovery options,
/// and low-level file management parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionWalConfig {
    /// Operational mode for WAL write operations (insert/update/delete)
    pub write_mode:             WalFailureMode,
    /// Operational mode for WAL verification operations
    pub verification_mode:      WalFailureMode,
    /// Whether to automatically verify documents against WAL on read
    pub auto_verify:            bool,
    /// Whether to enable WAL-based recovery features
    pub enable_recovery:        bool,
    /// Optional maximum WAL file size in bytes
    pub max_wal_size_bytes:     Option<u64>,
    /// Optional compression algorithm for rotated WAL files
    pub compression_algorithm:  Option<crate::CompressionAlgorithm>,
    /// Optional maximum number of records per WAL file
    pub max_records_per_file:   Option<usize>,
    /// WAL file format
    pub format:                 crate::manager::WalFormat,
}

impl Default for CollectionWalConfig {
    fn default() -> Self {
        Self {
            write_mode:             WalFailureMode::Strict,
            verification_mode:      WalFailureMode::Warn,
            auto_verify:            false,
            enable_recovery:        true,
            max_wal_size_bytes:     Some(10 * 1024 * 1024), // 10MB
            compression_algorithm:  Some(crate::CompressionAlgorithm::Zstd),
            max_records_per_file:   Some(1000),
            format:                 crate::manager::WalFormat::default(),
        }
    }
}

/// Configuration for WAL operations at the store level.
///
/// This struct defines global WAL settings that apply to all collections
/// in the store, with collection-specific overrides possible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreWalConfig {
    /// Default WAL configuration for collections
    pub default_collection_config: CollectionWalConfig,
    /// Collection-specific WAL configurations (overrides defaults)
    pub collection_configs:        HashMap<String, CollectionWalConfig>,
    /// Failure handling mode for store-level WAL operations (checkpoints, etc.)
    pub store_failure_mode:        WalFailureMode,
    /// Whether to enable automatic store-wide checkpoints
    pub auto_checkpoint:           bool,
    /// Interval for automatic checkpoints (in seconds, 0 = disabled)
    pub checkpoint_interval_secs:  u64,
    /// Maximum WAL file size before forcing checkpoint (in bytes)
    pub max_wal_size_bytes:        u64,
}

impl Default for StoreWalConfig {
    fn default() -> Self {
        Self {
            default_collection_config: CollectionWalConfig::default(),
            collection_configs:        HashMap::new(),
            store_failure_mode:        WalFailureMode::Strict,
            auto_checkpoint:           true,
            checkpoint_interval_secs:  300,               // 5 minutes
            max_wal_size_bytes:        100 * 1024 * 1024, // 100MB
        }
    }
}

impl From<CollectionWalConfig> for crate::manager::WalConfig {
    fn from(config: CollectionWalConfig) -> Self {
        Self {
            max_file_size:         config.max_wal_size_bytes,
            compression_algorithm: config.compression_algorithm,
            max_records_per_file:  config.max_records_per_file,
            format:                config.format,
        }
    }
}