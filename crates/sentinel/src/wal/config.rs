//! WAL configuration structures and operational modes.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Operational modes for WAL operations.
///
/// These modes control how WAL-related failures are handled:
/// - `Disabled`: WAL operations are skipped entirely
/// - `Warn`: WAL failures are logged as warnings but don't fail operations
/// - `Strict`: WAL failures cause operations to fail immediately
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WalMode {
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
/// including operational modes, verification settings, and recovery options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionWalConfig {
    /// Operational mode for WAL write operations (insert/update/delete)
    pub write_mode:         WalMode,
    /// Operational mode for WAL verification operations
    pub verification_mode:  WalMode,
    /// Whether to automatically verify documents against WAL on read
    pub auto_verify:        bool,
    /// Whether to enable WAL-based recovery features
    pub enable_recovery:    bool,
    /// Maximum number of WAL entries to keep in memory for verification
    pub max_cached_entries: usize,
}

impl Default for CollectionWalConfig {
    fn default() -> Self {
        Self {
            write_mode:         WalMode::Strict,
            verification_mode:  WalMode::Warn,
            auto_verify:        false,
            enable_recovery:    true,
            max_cached_entries: 1000,
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
    /// Operational mode for store-level WAL operations (checkpoints, etc.)
    pub store_mode:                WalMode,
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
            store_mode:                WalMode::Strict,
            auto_checkpoint:           true,
            checkpoint_interval_secs:  300,               // 5 minutes
            max_wal_size_bytes:        100 * 1024 * 1024, // 100MB
        }
    }
}
