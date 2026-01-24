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

impl std::str::FromStr for WalFailureMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "disabled" => Ok(Self::Disabled),
            "warn" => Ok(Self::Warn),
            "strict" => Ok(Self::Strict),
            _ => Err(format!("Invalid WAL failure mode: {}", s)),
        }
    }
}

impl std::fmt::Display for WalFailureMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Disabled => write!(f, "disabled"),
            Self::Warn => write!(f, "warn"),
            Self::Strict => write!(f, "strict"),
        }
    }
}

/// Configuration for WAL operations at the collection level.
///
/// This struct defines how WAL should behave for a specific collection,
/// including operational modes, verification settings, recovery options,
/// and low-level file management parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionWalConfig {
    /// Operational mode for WAL write operations (insert/update/delete)
    pub write_mode:            WalFailureMode,
    /// Operational mode for WAL verification operations
    pub verification_mode:     WalFailureMode,
    /// Whether to automatically verify documents against WAL on read
    pub auto_verify:           bool,
    /// Whether to enable WAL-based recovery features
    pub enable_recovery:       bool,
    /// Optional maximum WAL file size in bytes
    pub max_wal_size_bytes:    Option<u64>,
    /// Optional compression algorithm for rotated WAL files
    pub compression_algorithm: Option<crate::CompressionAlgorithm>,
    /// Optional maximum number of records per WAL file
    pub max_records_per_file:  Option<usize>,
    /// WAL file format
    pub format:                crate::manager::WalFormat,
}

impl Default for CollectionWalConfig {
    fn default() -> Self {
        Self {
            write_mode:            WalFailureMode::Strict,
            verification_mode:     WalFailureMode::Warn,
            auto_verify:           false,
            enable_recovery:       true,
            max_wal_size_bytes:    Some(10 * 1024 * 1024), // 10MB
            compression_algorithm: Some(crate::CompressionAlgorithm::Zstd),
            max_records_per_file:  Some(1000),
            format:                crate::manager::WalFormat::default(),
        }
    }
}

/// Overrides for CollectionWalConfig, where None means "use existing value".
#[derive(Debug, Clone, Default)]
pub struct CollectionWalConfigOverrides {
    pub write_mode:            Option<WalFailureMode>,
    pub verification_mode:     Option<WalFailureMode>,
    pub auto_verify:           Option<bool>,
    pub enable_recovery:       Option<bool>,
    pub max_wal_size_bytes:    Option<Option<u64>>, // None means don't override, Some(None) means set to None
    pub compression_algorithm: Option<Option<crate::CompressionAlgorithm>>,
    pub max_records_per_file:  Option<Option<usize>>,
    pub format:                Option<crate::manager::WalFormat>,
    /// Whether to persist the merged configuration to disk (for existing collections)
    pub persist_overrides:     bool,
}

impl CollectionWalConfig {
    /// Apply overrides to this config, returning a new config with overrides applied.
    pub fn apply_overrides(&self, overrides: &CollectionWalConfigOverrides) -> Self {
        Self {
            write_mode:            overrides.write_mode.unwrap_or(self.write_mode),
            verification_mode:     overrides
                .verification_mode
                .unwrap_or(self.verification_mode),
            auto_verify:           overrides.auto_verify.unwrap_or(self.auto_verify),
            enable_recovery:       overrides.enable_recovery.unwrap_or(self.enable_recovery),
            max_wal_size_bytes:    overrides
                .max_wal_size_bytes
                .unwrap_or(self.max_wal_size_bytes),
            compression_algorithm: overrides
                .compression_algorithm
                .unwrap_or(self.compression_algorithm),
            max_records_per_file:  overrides
                .max_records_per_file
                .unwrap_or(self.max_records_per_file),
            format:                overrides.format.unwrap_or(self.format),
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
