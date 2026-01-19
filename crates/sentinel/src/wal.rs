//! WAL (Write-Ahead Logging) configuration and operations for Sentinel DBMS.
//!
//! This module provides comprehensive WAL functionality including:
//! - Configuration management for WAL operations
//! - Checkpoint operations for collections and stores
//! - WAL streaming and verification
//! - Collection recovery from WAL
//! - Automatic document verification against WAL

use std::{collections::HashMap, pin::Pin};

use async_stream::stream;
use futures::Stream;
use serde::{Deserialize, Serialize};
use tokio::fs as tokio_fs;
use tracing::{debug, error, warn};
use sentinel_wal::{EntryType, LogEntry};

use crate::{Collection, Result, SentinelError, Store};

/// Operational modes for WAL operations.
///
/// These modes control how WAL-related failures are handled:
/// - `Disabled`: WAL operations are skipped entirely
/// - `Warn`: WAL failures are logged as warnings but don't fail operations
/// - `Strict`: WAL failures cause operations to fail immediately
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WalMode {
    /// WAL operations are completely disabled
    Disabled,
    /// WAL failures are logged as warnings but operations continue
    Warn,
    /// WAL failures cause operations to fail (default for data integrity)
    Strict,
}

impl Default for WalMode {
    fn default() -> Self { Self::Strict }
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

/// Collection metadata stored on disk.
///
/// This struct contains all persistent metadata for a collection,
/// including WAL configuration and operational state.
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
        }
    }

    /// Update the last modification timestamp
    pub fn touch(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

/// Store metadata stored on disk.
///
/// This struct contains all persistent metadata for the store,
/// including global WAL configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMetadata {
    /// Store creation timestamp
    pub created_at:  u64,
    /// Last modification timestamp
    pub updated_at:  u64,
    /// Global WAL configuration
    pub wal_config:  StoreWalConfig,
    /// List of collection names
    pub collections: Vec<String>,
}

impl StoreMetadata {
    /// Create new metadata for a store
    pub fn new(wal_config: StoreWalConfig) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            created_at: now,
            updated_at: now,
            wal_config,
            collections: Vec::new(),
        }
    }

    /// Update the last modification timestamp
    pub fn touch(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Add a collection to the store
    pub fn add_collection(&mut self, name: &str) {
        if !self.collections.iter().any(|c| c == name) {
            self.collections.push(name.to_string());
            self.touch();
        }
    }

    /// Remove a collection from the store
    pub fn remove_collection(&mut self, name: &str) {
        self.collections.retain(|c| c != name);
        self.touch();
    }
}

/// Extension trait for Store to add WAL operations
#[async_trait::async_trait]
pub trait StoreWalOps {
    /// Get the WAL configuration for the store
    async fn wal_config(&self) -> Result<StoreWalConfig>;

    /// Set the WAL configuration for the store
    async fn set_wal_config(&self, config: StoreWalConfig) -> Result<()>;

    /// Get the WAL configuration for a specific collection
    async fn collection_wal_config(&self, collection_name: &str) -> Result<CollectionWalConfig>;

    /// Set the WAL configuration for a specific collection
    async fn set_collection_wal_config(&self, collection_name: &str, config: CollectionWalConfig) -> Result<()>;

    /// Perform a checkpoint on all collections in the store
    async fn checkpoint_all_collections(&self) -> Result<()>;

    /// Stream WAL entries for all collections in the store
    async fn stream_all_wal_entries(&self) -> Result<impl Stream<Item = Result<(String, LogEntry)>> + Send>;

    /// Verify all collections against their WAL files
    async fn verify_all_collections(&self) -> Result<HashMap<String, Vec<WalVerificationIssue>>>;

    /// Recover all collections from their WAL files
    async fn recover_all_collections(&self) -> Result<HashMap<String, usize>>;
}

/// WAL verification issue
#[derive(Debug, Clone)]
pub struct WalVerificationIssue {
    /// The transaction ID where the issue occurred
    pub transaction_id: String,
    /// The document ID affected
    pub document_id:    String,
    /// Description of the issue
    pub description:    String,
    /// Whether this is a critical issue
    pub is_critical:    bool,
}

/// Extension trait for Collection to add WAL operations
#[async_trait::async_trait]
pub trait CollectionWalOps {
    /// Get the WAL configuration for this collection
    async fn wal_config(&self) -> Result<CollectionWalConfig>;

    /// Set the WAL configuration for this collection
    async fn set_wal_config(&self, config: CollectionWalConfig) -> Result<()>;

    /// Perform a checkpoint on this collection's WAL
    async fn checkpoint_wal(&self) -> Result<()>;

    /// Stream WAL entries for this collection
    async fn stream_wal_entries(&self) -> Result<impl Stream<Item = Result<LogEntry>> + Send + '_>;

    /// Verify this collection against its WAL file
    async fn verify_against_wal(&self) -> Result<Vec<WalVerificationIssue>>;

    /// Recover this collection from its WAL file
    async fn recover_from_wal(&self) -> Result<usize>;

    /// Get the current WAL size in bytes
    async fn wal_size(&self) -> Result<u64>;

    /// Get the number of entries in the WAL
    async fn wal_entries_count(&self) -> Result<usize>;
}

#[async_trait::async_trait]
impl StoreWalOps for Store {
    async fn wal_config(&self) -> Result<StoreWalConfig> {
        let metadata_path = self.root_path().join(".metadata.json");
        if tokio_fs::try_exists(&metadata_path).await.map_err(|e| {
            error!("Failed to check store metadata file: {}", e);
            e
        })? {
            let content = tokio_fs::read_to_string(&metadata_path)
                .await
                .map_err(|e| {
                    error!("Failed to read store metadata: {}", e);
                    e
                })?;
            let metadata: StoreMetadata = serde_json::from_str(&content).map_err(|e| {
                error!("Failed to parse store metadata: {}", e);
                SentinelError::StoreCorruption {
                    reason: format!("invalid store metadata JSON: {}", e),
                }
            })?;
            Ok(metadata.wal_config)
        }
        else {
            // Return default config if no metadata exists
            Ok(StoreWalConfig::default())
        }
    }

    async fn set_wal_config(&self, config: StoreWalConfig) -> Result<()> {
        let metadata_path = self.root_path().join(".metadata.json");
        let mut metadata = if tokio_fs::try_exists(&metadata_path).await.map_err(|e| {
            error!("Failed to check store metadata file: {}", e);
            e
        })? {
            let content = tokio_fs::read_to_string(&metadata_path)
                .await
                .map_err(|e| {
                    error!("Failed to read store metadata: {}", e);
                    e
                })?;
            serde_json::from_str(&content).map_err(|e| {
                error!("Failed to parse store metadata: {}", e);
                SentinelError::StoreCorruption {
                    reason: format!("invalid store metadata JSON: {}", e),
                }
            })?
        }
        else {
            StoreMetadata::new(config.clone())
        };

        metadata.wal_config = config;
        metadata.touch();

        let content = serde_json::to_string_pretty(&metadata).map_err(|e| {
            error!("Failed to serialize store metadata: {}", e);
            e
        })?;

        tokio_fs::write(&metadata_path, content)
            .await
            .map_err(|e| {
                error!("Failed to write store metadata: {}", e);
                e
            })?;

        debug!("Store WAL configuration updated");
        Ok(())
    }

    async fn collection_wal_config(&self, collection_name: &str) -> Result<CollectionWalConfig> {
        let store_config = self.wal_config().await?;
        Ok(store_config
            .collection_configs
            .get(collection_name)
            .cloned()
            .unwrap_or(store_config.default_collection_config))
    }

    async fn set_collection_wal_config(&self, collection_name: &str, config: CollectionWalConfig) -> Result<()> {
        let mut store_config = self.wal_config().await?;
        store_config
            .collection_configs
            .insert(collection_name.to_string(), config);
        self.set_wal_config(store_config).await
    }

    async fn checkpoint_all_collections(&self) -> Result<()> {
        let store_config = self.wal_config().await?;
        if store_config.store_mode == WalMode::Disabled {
            debug!("Store WAL mode is disabled, skipping checkpoint");
            return Ok(());
        }

        let collections = self.list_collections().await?;
        for collection_name in collections {
            let collection = self.collection(&collection_name).await?;
            if let Err(e) = collection.checkpoint_wal().await {
                match store_config.store_mode {
                    WalMode::Strict => {
                        error!("Failed to checkpoint collection {}: {}", collection_name, e);
                        return Err(e);
                    },
                    WalMode::Warn => {
                        warn!("Failed to checkpoint collection {}: {}", collection_name, e);
                    },
                    WalMode::Disabled => unreachable!(),
                }
            }
        }

        debug!("Checkpoint completed for all collections");
        Ok(())
    }

    async fn stream_all_wal_entries(&self) -> Result<Pin<Box<dyn Stream<Item = Result<(String, LogEntry)>> + Send>>> {
        let collections = self.list_collections().await?;
        let mut streams = Vec::new();

        for collection_name in collections {
            let collection = self.collection(&collection_name).await?;
            if let Ok(stream) = collection.stream_wal_entries().await {
                let collection_name_clone = collection_name.clone();
                let mapped_stream = futures::StreamExt::map(stream, move |entry: Result<LogEntry>| {
                    entry.map(|e| (collection_name_clone.clone(), e))
                });
                streams.push(Box::pin(mapped_stream));
            }
        }

        Ok(Box::pin(futures::stream::select_all(streams)))
    }

    async fn verify_all_collections(&self) -> Result<HashMap<String, Vec<WalVerificationIssue>>> {
        let collections = self.list_collections().await?;
        let mut results = HashMap::new();

        for collection_name in collections {
            let collection = self.collection(&collection_name).await?;
            match collection.verify_against_wal().await {
                Ok(issues) => {
                    if !issues.is_empty() {
                        results.insert(collection_name, issues);
                    }
                },
                Err(e) => {
                    let store_config = self.wal_config().await?;
                    match store_config.store_mode {
                        WalMode::Strict => return Err(e),
                        WalMode::Warn => {
                            warn!("Failed to verify collection {}: {}", collection_name, e);
                            results.insert(
                                collection_name,
                                vec![WalVerificationIssue {
                                    transaction_id: "unknown".to_string(),
                                    document_id:    "unknown".to_string(),
                                    description:    format!("Verification failed: {}", e),
                                    is_critical:    true,
                                }],
                            );
                        },
                        WalMode::Disabled => {},
                    }
                },
            }
        }

        Ok(results)
    }

    async fn recover_all_collections(&self) -> Result<HashMap<String, usize>> {
        let collections = self.list_collections().await?;
        let mut results = HashMap::new();

        for collection_name in collections {
            let collection = self.collection(&collection_name).await?;
            match CollectionWalOps::recover_from_wal(&collection).await {
                Ok(count) => {
                    results.insert(collection_name, count);
                },
                Err(e) => {
                    let store_config = self.wal_config().await?;
                    match store_config.store_mode {
                        WalMode::Strict => return Err(e),
                        WalMode::Warn => {
                            warn!("Failed to recover collection {}: {}", collection_name, e);
                        },
                        WalMode::Disabled => {},
                    }
                },
            }
        }

        Ok(results)
    }
}

#[async_trait::async_trait]
impl CollectionWalOps for Collection {
    async fn wal_config(&self) -> Result<CollectionWalConfig> {
        let metadata_path = self.path.join(".metadata.json");
        if tokio_fs::try_exists(&metadata_path).await? {
            let content = tokio_fs::read_to_string(&metadata_path).await?;
            let metadata: CollectionMetadata = serde_json::from_str(&content).map_err(|e| {
                SentinelError::StoreCorruption {
                    reason: format!("invalid collection metadata JSON: {}", e),
                }
            })?;
            Ok(metadata.wal_config)
        }
        else {
            // Return default config if no metadata exists
            Ok(CollectionWalConfig::default())
        }
    }

    async fn set_wal_config(&self, config: CollectionWalConfig) -> Result<()> {
        let metadata_path = self.path.join(".metadata.json");
        let mut metadata = if tokio_fs::try_exists(&metadata_path).await? {
            let content = tokio_fs::read_to_string(&metadata_path).await?;
            serde_json::from_str(&content).map_err(|e| {
                SentinelError::StoreCorruption {
                    reason: format!("invalid collection metadata JSON: {}", e),
                }
            })?
        }
        else {
            CollectionMetadata::new(self.name().to_string(), config.clone())
        };

        metadata.wal_config = config;
        metadata.touch();

        let content = serde_json::to_string_pretty(&metadata)?;
        tokio_fs::write(&metadata_path, content).await?;

        debug!("Collection {} WAL configuration updated", self.name());
        Ok(())
    }

    async fn checkpoint_wal(&self) -> Result<()> {
        if let Some(wal) = &self.wal_manager {
            wal.checkpoint().await?;
            debug!("WAL checkpoint completed for collection {}", self.name());
        }
        Ok(())
    }

    async fn stream_wal_entries(&self) -> Result<Pin<Box<dyn Stream<Item = Result<LogEntry>> + Send>>> {
        let wal_manager = self.wal_manager.clone();
        Ok(Box::pin(stream! {
            if let Some(wal) = wal_manager {
                let stream = wal.stream_entries();
                let mut pinned_stream = std::pin::pin!(stream);
                while let Some(entry_result) = futures::StreamExt::next(&mut pinned_stream).await {
                    yield entry_result.map_err(SentinelError::from);
                }
            }
        }))
    }

    async fn verify_against_wal(&self) -> Result<Vec<WalVerificationIssue>> {
        let mut issues = Vec::new();

        if let Some(wal) = &self.wal_manager {
            let stream = wal.stream_entries();
            let mut pinned_stream = std::pin::pin!(stream);
            while let Some(entry_result) = futures::StreamExt::next(&mut pinned_stream).await {
                match entry_result {
                    Ok(entry) => {
                        let issue = self.verify_wal_entry(&entry).await?;
                        if let Some(issue) = issue {
                            issues.push(issue);
                        }
                    },
                    Err(e) => {
                        issues.push(WalVerificationIssue {
                            transaction_id: "unknown".to_string(),
                            document_id:    "unknown".to_string(),
                            description:    format!("Failed to read WAL entry: {}", e),
                            is_critical:    true,
                        });
                    },
                }
            }
        }

        Ok(issues)
    }

    async fn recover_from_wal(&self) -> Result<usize> {
        let mut recovered_count = 0;

        if let Some(wal) = &self.wal_manager {
            let stream = wal.stream_entries();
            let mut pinned_stream = std::pin::pin!(stream);
            while let Some(entry_result) = futures::StreamExt::next(&mut pinned_stream).await {
                match entry_result {
                    Ok(entry) => {
                        if self.replay_wal_entry(&entry).await? {
                            recovered_count += 1;
                        }
                    },
                    Err(e) => {
                        error!("Failed to read WAL entry during recovery: {}", e);
                        return Err(SentinelError::from(e));
                    },
                }
            }
        }

        debug!(
            "Recovered {} entries for collection {}",
            recovered_count,
            self.name()
        );
        Ok(recovered_count)
    }

    async fn wal_size(&self) -> Result<u64> {
        if let Some(wal) = &self.wal_manager {
            Ok(wal.size().await?)
        }
        else {
            Ok(0)
        }
    }

    async fn wal_entries_count(&self) -> Result<usize> {
        if let Some(_wal) = &self.wal_manager {
            // This would need to be implemented in WalManager
            // For now, return 0
            Ok(0)
        }
        else {
            Ok(0)
        }
    }
}

impl Collection {
    /// Verify a single WAL entry against the current collection state
    async fn verify_wal_entry(&self, entry: &LogEntry) -> Result<Option<WalVerificationIssue>> {
        match entry.entry_type {
            EntryType::Insert | EntryType::Update => {
                if let Some(expected_data) = &entry.data {
                    match self.get(entry.document_id_str()).await {
                        Ok(Some(doc)) => {
                            let current_data = doc.data();
                            if current_data != expected_data {
                                return Ok(Some(WalVerificationIssue {
                                    transaction_id: entry.transaction_id_str().to_string(),
                                    document_id:    entry.document_id_str().to_string(),
                                    description:    format!(
                                        "Document data mismatch: WAL shows {:?}, collection has {:?}",
                                        expected_data, current_data
                                    ),
                                    is_critical:    true,
                                }));
                            }
                        },
                        Ok(None) => {
                            return Ok(Some(WalVerificationIssue {
                                transaction_id: entry.transaction_id_str().to_string(),
                                document_id:    entry.document_id_str().to_string(),
                                description:    "Document exists in WAL but not in collection".to_string(),
                                is_critical:    true,
                            }));
                        },
                        Err(e) => {
                            return Ok(Some(WalVerificationIssue {
                                transaction_id: entry.transaction_id_str().to_string(),
                                document_id:    entry.document_id_str().to_string(),
                                description:    format!("Failed to read document from collection: {}", e),
                                is_critical:    true,
                            }));
                        },
                    }
                }
            },
            EntryType::Delete => {
                match self.get(entry.document_id_str()).await {
                    Ok(Some(_)) => {
                        return Ok(Some(WalVerificationIssue {
                            transaction_id: entry.transaction_id_str().to_string(),
                            document_id:    entry.document_id_str().to_string(),
                            description:    "Document marked as deleted in WAL but still exists in collection"
                                .to_string(),
                            is_critical:    true,
                        }));
                    },
                    Ok(None) => {
                        // Check if it exists in .deleted
                        let deleted_path = self
                            .path
                            .join(".deleted")
                            .join(format!("{}.json", entry.document_id_str()));
                        if !tokio_fs::try_exists(&deleted_path).await? {
                            return Ok(Some(WalVerificationIssue {
                                transaction_id: entry.transaction_id_str().to_string(),
                                document_id:    entry.document_id_str().to_string(),
                                description:    "Document marked as deleted in WAL but not found in .deleted directory"
                                    .to_string(),
                                is_critical:    false,
                            }));
                        }
                    },
                    Err(e) => {
                        return Ok(Some(WalVerificationIssue {
                            transaction_id: entry.transaction_id_str().to_string(),
                            document_id:    entry.document_id_str().to_string(),
                            description:    format!("Failed to check document in collection: {}", e),
                            is_critical:    true,
                        }));
                    },
                }
            },
            // Transaction control entries don't affect document state
            EntryType::Begin | EntryType::Commit | EntryType::Rollback => {},
        }

        Ok(None)
    }

    /// Replay a single WAL entry to recover the collection state
    async fn replay_wal_entry(&self, entry: &LogEntry) -> Result<bool> {
        match entry.entry_type {
            EntryType::Insert => {
                if let Some(data_str) = &entry.data {
                    // Parse the JSON string to Value
                    let data: serde_json::Value = serde_json::from_str(data_str)?;
                    // Only insert if document doesn't exist
                    if self.get(entry.document_id_str()).await?.is_none() {
                        self.insert(entry.document_id_str(), data).await?;
                        Ok(true)
                    }
                    else {
                        Ok(false)
                    }
                }
                else {
                    Ok(false)
                }
            },
            EntryType::Update => {
                if let Some(data_str) = &entry.data {
                    // Parse the JSON string to Value
                    let data: serde_json::Value = serde_json::from_str(data_str)?;
                    // Update existing document
                    self.update(entry.document_id_str(), data).await?;
                    Ok(true)
                }
                else {
                    Ok(false)
                }
            },
            EntryType::Delete => {
                // Only delete if document exists
                if self.get(entry.document_id_str()).await?.is_some() {
                    self.delete(entry.document_id_str()).await?;
                    Ok(true)
                }
                else {
                    Ok(false)
                }
            },
            // Transaction control entries don't affect document state
            EntryType::Begin | EntryType::Commit | EntryType::Rollback => Ok(false),
        }
    }
}
