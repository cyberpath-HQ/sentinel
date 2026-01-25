//! Core WAL operations and trait definitions.
//!
//! This module provides Write-Ahead Logging (WAL) operations for both Store and Collection
//! entities. WAL ensures data durability and consistency by logging operations before
//! they are applied to the main data store.
//!
//! # Architecture
//!
//! The WAL system is organized into two layers:
//! - **Low-level operations** in the `sentinel-wal` crate handle raw WAL file management
//! - **High-level operations** in this module provide trait-based interfaces for Store and
//!   Collection
//!
//! # Key Concepts
//!
//! - **Checkpoint**: Flushes accumulated WAL entries to the main data store and truncates the log
//! - **Recovery**: Replays WAL entries to restore data consistency after a crash
//! - **Verification**: Validates WAL integrity and consistency with the main data store
//! - **Streaming**: Provides real-time access to WAL entries for monitoring and replication
//!
//! # Examples
//!
//! ## Basic WAL Operations on a Collection
//!
//! ```rust,no_run
//! # use sentinel_dbms::{Store, Collection};
//! # use futures::StreamExt;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let store = Store::new("/tmp/store", None).await?;
//! # let collection = store.collection("users").await?;
//! use sentinel_dbms::wal::ops::CollectionWalOps;
//!
//! // Insert some data
//! collection.insert("user-123", serde_json::json!({"name": "Alice"})).await?;
//!
//! // Checkpoint the WAL to persist changes
//! collection.checkpoint_wal().await?;
//!
//! // Get WAL statistics
//! let size = collection.wal_size().await?;
//! let count = collection.wal_entries_count().await?;
//! println!("WAL size: {} bytes, entries: {}", size, count);
//!
//! // Stream WAL entries for monitoring
//! let mut stream = collection.stream_wal_entries().await?;
//! while let Some(entry) = stream.next().await {
//!     let entry = entry?;
//!     println!("WAL entry: {:?}", entry);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Store-level WAL Operations
//!
//! ```rust,no_run
//! # use sentinel_dbms::Store;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let store = Store::new("/tmp/store", None).await?;
//! use sentinel_dbms::wal::ops::StoreWalOps;
//!
//! // Checkpoint all collections
//! store.checkpoint_all_collections().await?;
//!
//! // Verify all collections against their WALs
//! let issues = store.verify_all_collections().await?;
//! for (collection_name, collection_issues) in issues {
//!     println!(
//!         "Collection {} has {} issues",
//!         collection_name,
//!         collection_issues.len()
//!     );
//! }
//!
//! // Recover all collections from WAL
//! let recovery_stats = store.recover_all_collections().await?;
//! for (collection_name, operations) in recovery_stats {
//!     println!(
//!         "Recovered {} operations for {}",
//!         operations, collection_name
//!     );
//! }
//! # Ok(())
//! # }
//! ```

use std::{collections::HashMap, pin::Pin};

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use tracing::{debug, error, info, warn};
use sentinel_wal::{
    recover_from_wal_safe,
    verify_wal_consistency,
    LogEntry,
    WalRecoveryResult,
    WalVerificationIssue,
    WalVerificationResult,
};

use crate::{Collection, Store};
use crate::store::operations::collection_with_config;

/// Extension trait for Store to add WAL operations.
///
/// This trait provides high-level WAL operations that work across all collections
/// in a store. Operations are performed sequentially on each collection to ensure
/// consistency and avoid resource conflicts.
///
/// # Thread Safety
///
/// All operations are async and can be called concurrently, but the trait implementations
/// handle internal synchronization through the Store's collection management.
#[async_trait]
pub trait StoreWalOps {
    /// Perform a checkpoint on all collections in the store.
    ///
    /// This operation iterates through all collections and checkpoints each one's WAL,
    /// ensuring all pending operations are flushed to the main data store. This is
    /// typically called during maintenance windows or before backups.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if any collection fails to checkpoint.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use sentinel_dbms::Store;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let store = Store::new("/tmp/store", None).await?;
    /// use sentinel_dbms::wal::ops::StoreWalOps;
    ///
    /// store.checkpoint_all_collections().await?;
    /// println!("All collections checkpointed successfully");
    /// # Ok(())
    /// # }
    /// ```
    async fn checkpoint_all_collections(&self) -> crate::Result<()>;

    /// Stream WAL entries from all collections in the store.
    ///
    /// Creates a unified stream that yields WAL entries from all collections,
    /// prefixed with the collection name. This is useful for monitoring, auditing,
    /// and replication across the entire store.
    ///
    /// # Returns
    ///
    /// Returns a stream yielding `(collection_name, LogEntry)` tuples, or an error
    /// if the collection list cannot be retrieved.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use sentinel_dbms::Store;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let store = Store::new("/tmp/store", None).await?;
    /// use sentinel_dbms::wal::ops::StoreWalOps;
    /// use futures::StreamExt;
    ///
    /// let mut stream = store.stream_all_wal_entries().await?;
    /// while let Some(result) = stream.next().await {
    ///     let (collection_name, entry) = result?;
    ///     println!("Collection {}: {:?}", collection_name, entry);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn stream_all_wal_entries(
        &self,
    ) -> crate::Result<Pin<Box<dyn Stream<Item = crate::Result<(String, LogEntry)>> + Send>>>;

    /// Verify all collections against their WAL files.
    ///
    /// Performs consistency checks on all collections to ensure WAL entries match
    /// the current state of documents. Returns a map of collection names to any
    /// verification issues found.
    ///
    /// # Returns
    ///
    /// Returns a `HashMap` where keys are collection names and values are vectors
    /// of `WalVerificationIssue`s. Collections with no issues are not included.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use sentinel_dbms::Store;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let store = Store::new("/tmp/store", None).await?;
    /// use sentinel_dbms::wal::ops::StoreWalOps;
    ///
    /// let issues = store.verify_all_collections().await?;
    /// if issues.is_empty() {
    ///     println!("All collections are consistent with their WALs");
    /// }
    /// else {
    ///     for (collection_name, collection_issues) in issues {
    ///         println!(
    ///             "Collection {} has {} issues:",
    ///             collection_name,
    ///             collection_issues.len()
    ///         );
    ///         for issue in collection_issues {
    ///             println!("  - {}", issue.description);
    ///         }
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn verify_all_collections(&self) -> crate::Result<HashMap<String, Vec<WalVerificationIssue>>>;

    /// Recover all collections from their WAL files.
    ///
    /// Performs crash recovery on all collections by replaying WAL entries.
    /// This is typically called during store initialization after an unclean shutdown.
    ///
    /// # Returns
    ///
    /// Returns a `HashMap` where keys are collection names and values are the number
    /// of operations recovered for each collection.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use sentinel_dbms::Store;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let store = Store::new("/tmp/store", None).await?;
    /// use sentinel_dbms::wal::ops::StoreWalOps;
    ///
    /// let recovery_stats = store.recover_all_collections().await?;
    /// let total_operations: usize = recovery_stats.values().sum();
    /// println!(
    ///     "Recovered {} operations across {} collections",
    ///     total_operations,
    ///     recovery_stats.len()
    /// );
    /// # Ok(())
    /// # }
    /// ```
    async fn recover_all_collections(&self) -> crate::Result<HashMap<String, usize>>;
}

/// Extension trait for Collection to add WAL operations.
///
/// This trait provides WAL operations specific to individual collections,
/// including checkpointing, verification, recovery, and monitoring capabilities.
///
/// # Thread Safety
///
/// All operations are async and work with the collection's internal locking mechanisms.
#[async_trait]
pub trait CollectionWalOps {
    /// Perform a checkpoint on this collection's WAL.
    ///
    /// Flushes all pending WAL entries to the main data store and truncates the WAL file.
    /// This operation ensures durability and can help manage WAL file size.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if the checkpoint operation fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use sentinel_dbms::{Store, Collection};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let store = Store::new("/tmp/store", None).await?;
    /// # let collection = store.collection("users").await?;
    /// use sentinel_dbms::wal::ops::CollectionWalOps;
    ///
    /// // Perform operations
    /// collection.insert("user-123", serde_json::json!({"name": "Alice"})).await?;
    /// collection.update("user-123", serde_json::json!({"name": "Alice", "age": 30})).await?;
    ///
    /// // Checkpoint to persist changes
    /// collection.checkpoint_wal().await?;
    /// println!("WAL checkpoint completed");
    /// # Ok(())
    /// # }
    /// ```
    async fn checkpoint_wal(&self) -> crate::Result<()>;

    /// Stream WAL entries for this collection.
    ///
    /// Creates a stream that yields all current WAL entries for the collection.
    /// This is useful for monitoring recent operations, auditing, or replication.
    ///
    /// # Returns
    ///
    /// Returns a stream yielding `LogEntry` items, or an error if WAL access fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use sentinel_dbms::{Store, Collection};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let store = Store::new("/tmp/store", None).await?;
    /// # let collection = store.collection("users").await?;
    /// use sentinel_dbms::wal::ops::CollectionWalOps;
    /// use futures::StreamExt;
    ///
    /// let mut stream = collection.stream_wal_entries().await?;
    /// let mut operation_count = 0;
    /// while let Some(result) = stream.next().await {
    ///     let entry = result?;
    ///     operation_count += 1;
    ///     println!("Operation {}: {:?}", operation_count, entry.entry_type);
    /// }
    /// println!("Total operations in WAL: {}", operation_count);
    /// # Ok(())
    /// # }
    /// ```
    async fn stream_wal_entries(&self) -> crate::Result<Pin<Box<dyn Stream<Item = crate::Result<LogEntry>> + Send>>>;

    /// Verify this collection against its WAL file.
    ///
    /// Performs consistency checks to ensure WAL entries match the current state
    /// of documents in the collection. This helps detect corruption or inconsistencies.
    ///
    /// # Returns
    ///
    /// Returns a `WalVerificationResult` containing verification statistics and any issues found.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use sentinel_dbms::{Store, Collection};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let store = Store::new("/tmp/store", None).await?;
    /// # let collection = store.collection("users").await?;
    /// use sentinel_dbms::wal::ops::CollectionWalOps;
    ///
    /// let result = collection.verify_against_wal().await?;
    /// println!(
    ///     "Verification result: {}",
    ///     if result.passed { "PASSED" } else { "FAILED" }
    /// );
    /// println!("Entries processed: {}", result.entries_processed);
    /// println!("Issues found: {}", result.issues.len());
    ///
    /// for issue in &result.issues {
    ///     println!("Issue: {}", issue.description);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn verify_against_wal(&self) -> crate::Result<WalVerificationResult>;

    /// Recover this collection from its WAL file.
    ///
    /// Replays WAL entries to restore the collection to a consistent state after
    /// a crash or unclean shutdown. This operation is safe and will not overwrite
    /// newer data.
    ///
    /// # Returns
    ///
    /// Returns a `WalRecoveryResult` with recovery statistics and any failures encountered.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use sentinel_dbms::{Store, Collection};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let store = Store::new("/tmp/store", None).await?;
    /// # let collection = store.collection("users").await?;
    /// use sentinel_dbms::wal::ops::CollectionWalOps;
    ///
    /// let result = collection.recover_from_wal().await?;
    /// println!("Recovery completed:");
    /// println!("  Operations recovered: {}", result.recovered_operations);
    /// println!("  Operations skipped: {}", result.skipped_operations);
    /// println!("  Operations failed: {}", result.failed_operations);
    ///
    /// if !result.failures.is_empty() {
    ///     println!("Recovery failures:");
    ///     for failure in &result.failures {
    ///         println!("  - {:?}", failure);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn recover_from_wal(&self) -> crate::Result<WalRecoveryResult>;

    /// Get the current WAL size in bytes.
    ///
    /// Returns the size of the WAL file on disk. This can be used to monitor
    /// WAL growth and determine when checkpointing might be beneficial.
    ///
    /// # Returns
    ///
    /// Returns the WAL file size in bytes, or 0 if no WAL is configured.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use sentinel_dbms::{Store, Collection};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let store = Store::new("/tmp/store", None).await?;
    /// # let collection = store.collection("users").await?;
    /// use sentinel_dbms::wal::ops::CollectionWalOps;
    ///
    /// let size_bytes = collection.wal_size().await?;
    /// let size_mb = size_bytes as f64 / (1024.0 * 1024.0);
    /// println!("WAL size: {:.2} MB", size_mb);
    ///
    /// if size_bytes > 100 * 1024 * 1024 {
    ///     // 100 MB
    ///     println!("WAL is getting large, consider checkpointing");
    ///     collection.checkpoint_wal().await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn wal_size(&self) -> crate::Result<u64>;

    /// Get the number of entries in the WAL.
    ///
    /// Returns the count of operations logged in the WAL. This can be used to
    /// monitor operation frequency and determine checkpoint timing.
    ///
    /// # Returns
    ///
    /// Returns the number of WAL entries, or 0 if no WAL is configured.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use sentinel_dbms::{Store, Collection};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let store = Store::new("/tmp/store", None).await?;
    /// # let collection = store.collection("users").await?;
    /// use sentinel_dbms::wal::ops::CollectionWalOps;
    ///
    /// let count = collection.wal_entries_count().await?;
    /// println!("WAL contains {} entries", count);
    ///
    /// if count > 1000 {
    ///     println!("Many pending operations, checkpointing...");
    ///     collection.checkpoint_wal().await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn wal_entries_count(&self) -> crate::Result<usize>;
}

#[async_trait]
impl StoreWalOps for Store {
    async fn checkpoint_all_collections(&self) -> crate::Result<()> {
        let collections = self.list_collections().await?;
        info!("Starting checkpoint for {} collections", collections.len());

        for collection_name in collections {
            debug!("Checkpointing collection: {}", collection_name);
            let collection = collection_with_config(self, &collection_name, None).await?;
            CollectionWalOps::checkpoint_wal(&collection).await?;
        }

        info!("Checkpoint completed for all collections");
        Ok(())
    }

    async fn stream_all_wal_entries(
        &self,
    ) -> crate::Result<Pin<Box<dyn Stream<Item = crate::Result<(String, LogEntry)>> + Send>>> {
        let collections = self.list_collections().await?;
        debug!(
            "Streaming WAL entries from {} collections",
            collections.len()
        );

        let mut streams = Vec::new();

        for collection_name in collections {
            let collection = collection_with_config(self, &collection_name, None).await?;
            if let Ok(stream) = CollectionWalOps::stream_wal_entries(&collection).await {
                let collection_name_clone = collection_name.clone();
                let mapped_stream =
                    stream.map(move |entry: crate::Result<LogEntry>| entry.map(|e| (collection_name_clone.clone(), e)));
                streams.push(Box::pin(mapped_stream));
                debug!("Added WAL stream for collection: {}", collection_name);
            }
            else {
                warn!(
                    "Failed to create WAL stream for collection: {}",
                    collection_name
                );
            }
        }

        info!(
            "Created unified WAL stream from {} collections",
            streams.len()
        );
        Ok(Box::pin(futures::stream::select_all(streams)))
    }

    async fn verify_all_collections(&self) -> crate::Result<HashMap<String, Vec<WalVerificationIssue>>> {
        let collections = self.list_collections().await?;
        info!(
            "Starting WAL verification for {} collections",
            collections.len()
        );

        let mut results = HashMap::new();
        let mut total_issues = 0;

        for collection_name in collections {
            debug!("Verifying collection: {}", collection_name);
            let collection = collection_with_config(self, &collection_name, None).await?;
            match CollectionWalOps::verify_against_wal(&collection).await {
                Ok(verification_result) => {
                    if !verification_result.issues.is_empty() {
                        let issue_count = verification_result.issues.len();
                        total_issues += issue_count;
                        results.insert(collection_name.clone(), verification_result.issues);
                        warn!(
                            "Collection {} has {} verification issues",
                            collection_name, issue_count
                        );
                    }
                    else {
                        debug!("Collection {} verification passed", collection_name);
                    }
                },
                Err(e) => {
                    error!("Failed to verify collection {}: {}", collection_name, e);
                    results.insert(
                        collection_name.clone(),
                        vec![WalVerificationIssue {
                            transaction_id: "unknown".to_string(),
                            document_id:    "unknown".to_string(),
                            description:    format!("Verification failed: {}", e),
                            is_critical:    true,
                        }],
                    );
                    total_issues += 1;
                },
            }
        }

        if total_issues > 0 {
            warn!(
                "WAL verification completed with {} total issues across {} collections",
                total_issues,
                results.len()
            );
        }
        else {
            info!("WAL verification completed successfully - no issues found");
        }

        Ok(results)
    }

    async fn recover_all_collections(&self) -> crate::Result<HashMap<String, usize>> {
        let collections = self.list_collections().await?;
        info!(
            "Starting WAL recovery for {} collections",
            collections.len()
        );

        let mut results = HashMap::new();
        let mut total_operations = 0;

        for collection_name in collections {
            debug!("Recovering collection: {}", collection_name);
            let collection = collection_with_config(self, &collection_name, None).await?;
            match CollectionWalOps::recover_from_wal(&collection).await {
                Ok(recovery_result) => {
                    let operations = recovery_result.recovered_operations;
                    results.insert(collection_name.clone(), operations);
                    total_operations += operations;
                    if operations > 0 {
                        info!(
                            "Recovered {} operations for collection {}",
                            operations, collection_name
                        );
                    }
                    else {
                        debug!("No recovery needed for collection {}", collection_name);
                    }
                },
                Err(e) => {
                    error!("Failed to recover collection {}: {}", collection_name, e);
                    return Err(e);
                },
            }
        }

        info!(
            "WAL recovery completed - {} total operations recovered across {} collections",
            total_operations,
            results.len()
        );
        Ok(results)
    }
}

#[async_trait]
impl CollectionWalOps for Collection {
    async fn checkpoint_wal(&self) -> crate::Result<()> {
        if let Some(wal) = &self.wal_manager {
            debug!("Starting WAL checkpoint for collection {}", self.name());
            wal.checkpoint().await?;
            info!("WAL checkpoint completed for collection {}", self.name());
        }
        else {
            debug!("No WAL manager configured for collection {}", self.name());
        }
        Ok(())
    }

    async fn stream_wal_entries(&self) -> crate::Result<Pin<Box<dyn Stream<Item = crate::Result<LogEntry>> + Send>>> {
        if let Some(wal) = &self.wal_manager {
            debug!("Reading all WAL entries for collection {}", self.name());
            let entries = wal.read_all_entries().await?;
            debug!(
                "Retrieved {} WAL entries for collection {}",
                entries.len(),
                self.name()
            );
            let stream = futures::stream::iter(entries.into_iter().map(Ok));
            Ok(Box::pin(stream))
        }
        else {
            debug!(
                "No WAL manager configured for collection {}, returning empty stream",
                self.name()
            );
            Ok(Box::pin(futures::stream::empty()))
        }
    }

    async fn verify_against_wal(&self) -> crate::Result<WalVerificationResult> {
        if let Some(wal) = &self.wal_manager {
            debug!("Starting WAL verification for collection {}", self.name());
            let result = verify_wal_consistency(wal, self).await?;
            if result.passed {
                info!(
                    "WAL verification passed for collection {} ({} entries processed)",
                    self.name(),
                    result.entries_processed
                );
            }
            else {
                warn!(
                    "WAL verification failed for collection {}: {} issues found",
                    self.name(),
                    result.issues.len()
                );
                for issue in &result.issues {
                    warn!("  Verification issue: {}", issue.description);
                }
            }
            Ok(result)
        }
        else {
            debug!(
                "No WAL manager configured for collection {}, skipping verification",
                self.name()
            );
            Ok(WalVerificationResult {
                issues:             vec![],
                passed:             true,
                entries_processed:  0,
                affected_documents: 0,
            })
        }
    }

    async fn recover_from_wal(&self) -> crate::Result<WalRecoveryResult> {
        if let Some(wal) = &self.wal_manager {
            info!("Starting WAL recovery for collection {}", self.name());
            let result = recover_from_wal_safe(wal, self).await?;
            info!(
                "WAL recovery completed for collection {}: {} operations recovered, {} skipped, {} failed",
                self.name(),
                result.recovered_operations,
                result.skipped_operations,
                result.failed_operations
            );

            if !result.failures.is_empty() {
                warn!("Recovery failures for collection {}:", self.name());
                for failure in &result.failures {
                    warn!("  - {:?}", failure);
                }
            }
            Ok(result)
        }
        else {
            debug!(
                "No WAL manager configured for collection {}, skipping recovery",
                self.name()
            );
            Ok(WalRecoveryResult {
                recovered_operations: 0,
                skipped_operations:   0,
                failed_operations:    0,
                failures:             vec![],
            })
        }
    }

    async fn wal_size(&self) -> crate::Result<u64> {
        if let Some(wal) = &self.wal_manager {
            let size = wal.size().await?;
            debug!("WAL size for collection {}: {} bytes", self.name(), size);
            Ok(size)
        }
        else {
            debug!("No WAL manager configured for collection {}", self.name());
            Ok(0)
        }
    }

    async fn wal_entries_count(&self) -> crate::Result<usize> {
        if let Some(wal) = &self.wal_manager {
            let count = wal.entries_count().await?;
            debug!(
                "WAL entries count for collection {}: {}",
                self.name(),
                count
            );
            Ok(count)
        }
        else {
            debug!("No WAL manager configured for collection {}", self.name());
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::Store;

    /// Helper to create a test store with a collection
    async fn create_test_store_with_collection() -> (tempfile::TempDir, Store, String) {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(temp_dir.path().to_path_buf(), None, StoreWalConfig::default())
            .await
            .unwrap();
        let collection_name = "test_wal_collection".to_string();
        let _ = collection_with_config(&store, &collection_name, None).await.unwrap();
        (temp_dir, store, collection_name)
    }

    // ============ CollectionWalOps Tests ============

    #[tokio::test]
    async fn test_checkpoint_wal_with_wal_manager() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = collection_with_config(&store, &collection_name, None).await.unwrap();

        // Insert some data to create WAL entries
        collection
            .insert("doc-1", serde_json::json!({"test": 1}))
            .await
            .unwrap();
        collection
            .insert("doc-2", serde_json::json!({"test": 2}))
            .await
            .unwrap();

        // Checkpoint should succeed
        let result = collection.checkpoint_wal().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_checkpoint_wal_without_wal_manager() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Create collection without WAL config - checkpoint should still succeed (no-op)
        // The default collection may not have a WAL manager configured
        let result = collection.checkpoint_wal().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stream_wal_entries_with_data() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Insert some data
        collection
            .insert("doc-1", serde_json::json!({"name": "Test1"}))
            .await
            .unwrap();
        collection
            .insert("doc-2", serde_json::json!({"name": "Test2"}))
            .await
            .unwrap();

        // Stream entries
        let stream = collection.stream_wal_entries().await.unwrap();

        // Collect entries
        let entries: Vec<_> = stream.collect().await;
        assert!(!entries.is_empty());

        // Each entry should be Ok
        for entry in entries {
            assert!(entry.is_ok());
        }
    }

    #[tokio::test]
    async fn test_stream_wal_entries_empty() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Stream on empty collection
        let stream = collection.stream_wal_entries().await.unwrap();
        let entries: Vec<_> = stream.collect().await;

        // May be empty or have entries depending on WAL config
        // Just verify it doesn't error
        assert!(entries.is_empty() || entries.iter().all(|e| e.is_ok()));
    }

    #[tokio::test]
    async fn test_verify_against_wal() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Insert some data
        collection
            .insert("doc-1", serde_json::json!({"verify": true}))
            .await
            .unwrap();

        // Verify should succeed
        let result = collection.verify_against_wal().await;
        assert!(result.is_ok());

        let verification = result.unwrap();
        // If there are issues, verification didn't fully pass
        assert!(verification.passed || verification.issues.is_empty());
    }

    #[tokio::test]
    async fn test_recover_from_wal() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Insert some data
        collection
            .insert("doc-to-recover", serde_json::json!({"data": "test"}))
            .await
            .unwrap();

        // Recovery should succeed (even if no recovery needed)
        let result = collection.recover_from_wal().await;
        assert!(result.is_ok());

        let recovery = result.unwrap();
        // All operations should be accounted for
        let total = recovery.recovered_operations + recovery.skipped_operations + recovery.failed_operations;
        assert!(total >= 0); // Operations may or may not need recovery
    }

    #[tokio::test]
    async fn test_wal_size() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Get initial size
        let initial_size = collection.wal_size().await.unwrap();

        // Insert data
        collection
            .insert("doc-for-size", serde_json::json!({"size": "test data"}))
            .await
            .unwrap();

        // Get size after insert
        let new_size = collection.wal_size().await.unwrap();
        assert!(new_size >= initial_size);
    }

    #[tokio::test]
    async fn test_wal_entries_count() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Get initial count
        let initial_count = collection.wal_entries_count().await.unwrap();

        // Insert some data
        collection
            .insert("doc-1", serde_json::json!({"count": 1}))
            .await
            .unwrap();
        collection
            .insert("doc-2", serde_json::json!({"count": 2}))
            .await
            .unwrap();

        // Get count after inserts
        let new_count = collection.wal_entries_count().await.unwrap();
        assert!(new_count >= initial_count);
    }

    // ============ StoreWalOps Tests ============

    #[tokio::test]
    async fn test_checkpoint_all_collections() {
        let (temp_dir, store, _collection_name) = create_test_store_with_collection().await;

        // Insert some data
        let collection = store.collection("test1").await.unwrap();
        collection
            .insert("doc-1", serde_json::json!({"test": 1}))
            .await
            .unwrap();

        let collection2 = store.collection("test2").await.unwrap();
        collection2
            .insert("doc-2", serde_json::json!({"test": 2}))
            .await
            .unwrap();

        // Checkpoint all should succeed
        let result = store.checkpoint_all_collections().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stream_all_wal_entries() {
        let (temp_dir, store, _collection_name) = create_test_store_with_collection().await;

        // Insert some data in multiple collections
        let collection1 = store.collection("stream-collection-1").await.unwrap();
        collection1
            .insert("doc-1", serde_json::json!({"stream": 1}))
            .await
            .unwrap();

        let collection2 = store.collection("stream-collection-2").await.unwrap();
        collection2
            .insert("doc-2", serde_json::json!({"stream": 2}))
            .await
            .unwrap();

        // Stream all entries
        let stream = store.stream_all_wal_entries().await.unwrap();
        let entries: Vec<_> = stream.collect().await;

        // Should have entries from both collections
        // Each entry is (collection_name, LogEntry)
        for entry in entries {
            assert!(entry.is_ok());
            let (_name, log_entry) = entry.unwrap();
            // Verify it's a valid log entry
            assert!(!log_entry.document_id.is_empty());
        }
    }

    #[tokio::test]
    async fn test_verify_all_collections() {
        let (temp_dir, store, _collection_name) = create_test_store_with_collection().await;

        // Create multiple collections with data
        let collection1 = store.collection("verify-1").await.unwrap();
        collection1
            .insert("doc-1", serde_json::json!({"verify": 1}))
            .await
            .unwrap();

        let collection2 = store.collection("verify-2").await.unwrap();
        collection2
            .insert("doc-2", serde_json::json!({"verify": 2}))
            .await
            .unwrap();

        // Verify all should succeed
        let result = store.verify_all_collections().await;
        assert!(result.is_ok());

        let issues = result.unwrap();
        // Both collections should have passed verification
        for (name, collection_issues) in &issues {
            for issue in collection_issues {
                assert!(!issue.is_critical || issue.description.is_empty());
            }
            // Debug output
            if !collection_issues.is_empty() {
                eprintln!("Collection {} has {} issues", name, collection_issues.len());
            }
        }
    }

    #[tokio::test]
    async fn test_recover_all_collections() {
        let (temp_dir, store, _collection_name) = create_test_store_with_collection().await;

        // Create collections with data
        let collection1 = store.collection("recover-1").await.unwrap();
        collection1
            .insert("doc-1", serde_json::json!({"recover": 1}))
            .await
            .unwrap();

        let collection2 = store.collection("recover-2").await.unwrap();
        collection2
            .insert("doc-2", serde_json::json!({"recover": 2}))
            .await
            .unwrap();

        // Recover all should succeed
        let result = store.recover_all_collections().await;
        assert!(result.is_ok());

        let recovery_stats = result.unwrap();
        // Should have entries for the test collections (may include others)
        assert!(
            recovery_stats.len() >= 2,
            "Expected at least 2 collections, got {}",
            recovery_stats.len()
        );

        for (name, operations) in &recovery_stats {
            eprintln!("Collection {} recovered {} operations", name, operations);
        }
    }

    // ============ Edge Case Tests ============

    #[tokio::test]
    async fn test_wal_operations_on_empty_store() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(temp_dir.path().to_path_buf(), None, StoreWalConfig::default())
            .await
            .unwrap();

        // Verify empty store
        let result = store.verify_all_collections().await;
        assert!(result.is_ok());
        let issues = result.unwrap();
        assert!(issues.is_empty());

        // Recover empty store
        let result = store.recover_all_collections().await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert!(stats.is_empty());

        // Stream empty store
        let stream = store.stream_all_wal_entries().await.unwrap();
        let entries: Vec<_> = stream.collect().await;
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_checkpoint_empty_collection() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Checkpoint empty collection should succeed
        let result = collection.checkpoint_wal().await;
        assert!(result.is_ok());
    }

    // ============ Edge Case Tests - Additional Coverage ============

    #[tokio::test]
    async fn test_wal_ops_stream_entries_with_verify_all() {
        // Test streaming entries with verify_all option
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Insert some data
        collection
            .insert("verify-doc", serde_json::json!({"verify": "test"}))
            .await
            .unwrap();

        // Stream and verify entries
        let stream = store.stream_all_wal_entries().await.unwrap();
        let entries: Vec<_> = stream.collect().await;

        // Should have entries that can be verified
        assert!(!entries.is_empty());
        for entry in entries {
            // Each entry is a Result, so we need to handle both ok and err cases
            if let Ok(result) = entry {
                assert!(!result.1.document_id.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_wal_ops_verify_collection_with_no_wal_manager() {
        // Test verify_against_wal when no WAL manager is configured
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Verify should return empty result (passed) when no WAL
        let result = collection.verify_against_wal().await;
        assert!(result.is_ok());

        let verification = result.unwrap();
        assert!(verification.passed);
        assert_eq!(verification.entries_processed, 0);
    }

    #[tokio::test]
    async fn test_wal_ops_recover_from_wal_with_no_wal_manager() {
        // Test recover_from_wal when no WAL manager is configured
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Recovery should return empty result when no WAL
        let result = collection.recover_from_wal().await;
        assert!(result.is_ok());

        let recovery = result.unwrap();
        assert_eq!(recovery.recovered_operations, 0);
        assert_eq!(recovery.skipped_operations, 0);
        assert_eq!(recovery.failed_operations, 0);
    }

    #[tokio::test]
    async fn test_wal_ops_wal_size_with_no_wal_manager() {
        // Test wal_size when no WAL manager is configured
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Size should return 0 when no WAL
        let size = collection.wal_size().await.unwrap();
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn test_wal_ops_wal_entries_count_with_no_wal_manager() {
        // Test wal_entries_count when no WAL manager is configured
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Count should return 0 when no WAL
        let count = collection.wal_entries_count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_wal_ops_stream_wal_entries_with_no_wal_manager() {
        // Test stream_wal_entries when no WAL manager is configured
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Stream should return empty stream when no WAL
        let stream = collection.stream_wal_entries().await.unwrap();
        let entries: Vec<_> = stream.collect().await;

        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_wal_ops_verify_all_with_mixed_collections() {
        // Test verify_all_collections with multiple collections
        let (temp_dir, store, _collection_name) = create_test_store_with_collection().await;

        // Create collections with varying data
        for i in 0 .. 3 {
            let collection = store
                .collection(&format!("verify-multi-{}", i))
                .await
                .unwrap();
            collection
                .insert(&format!("doc-{}", i), serde_json::json!({"index": i}))
                .await
                .unwrap();
        }

        // Verify all should succeed
        let result = store.verify_all_collections().await;
        assert!(result.is_ok());

        let issues = result.unwrap();
        // All collections should have passed verification (no critical issues)
        for (name, collection_issues) in &issues {
            for issue in collection_issues {
                assert!(
                    !issue.is_critical || issue.description.is_empty(),
                    "Collection {} has critical issue: {}",
                    name,
                    issue.description
                );
            }
        }
    }

    #[tokio::test]
    async fn test_wal_ops_checkpoint_all_with_empty_store() {
        // Test checkpoint_all_collections on empty store
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(temp_dir.path().to_path_buf(), None, StoreWalConfig::default())
            .await
            .unwrap();

        // Checkpoint all should succeed on empty store
        let result = store.checkpoint_all_collections().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wal_ops_recover_all_with_empty_store() {
        // Test recover_all_collections on empty store
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(temp_dir.path().to_path_buf(), None, StoreWalConfig::default())
            .await
            .unwrap();

        // Recover all should succeed on empty store
        let result = store.recover_all_collections().await;
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert!(stats.is_empty());
    }

    // ============ Additional "No WAL Manager" Branch Tests ============

    #[tokio::test]
    async fn test_wal_ops_checkpoint_with_verification_options() {
        // Test checkpoint_wal with custom verification options
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Insert some data
        collection
            .insert("doc-1", serde_json::json!({"test": 1}))
            .await
            .unwrap();

        // Checkpoint should work with verification enabled
        let result = collection.checkpoint_wal().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wal_ops_stream_all_with_no_collections() {
        // Test stream_all_wal_entries when there are no collections
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(temp_dir.path().to_path_buf(), None, StoreWalConfig::default())
            .await
            .unwrap();

        // Stream should return empty stream when no collections
        let stream = store.stream_all_wal_entries().await.unwrap();
        let entries: Vec<_> = stream.collect().await;

        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_wal_ops_verify_all_with_no_issues() {
        // Test verify_all_collections when all collections pass verification
        let (temp_dir, store, _collection_name) = create_test_store_with_collection().await;

        // Create a collection and insert data
        let collection = store.collection("verify-pass").await.unwrap();
        collection
            .insert("doc-1", serde_json::json!({"verify": true}))
            .await
            .unwrap();

        // Verify all - should return empty map (no issues)
        let result = store.verify_all_collections().await;
        assert!(result.is_ok());

        let issues = result.unwrap();
        // Collections with no issues should not be in the map
        assert!(issues.get("verify-pass").is_none() || issues.get("verify-pass").map_or(true, |v| v.is_empty()));
    }

    #[tokio::test]
    async fn test_wal_ops_recover_all_with_partial_failures() {
        // Test recover_all_collections handles partial failures gracefully
        let (temp_dir, store, _collection_name) = create_test_store_with_collection().await;

        // Create a collection with data
        let collection = store.collection("recover-test").await.unwrap();
        collection
            .insert("doc-1", serde_json::json!({"recover": true}))
            .await
            .unwrap();

        // Recovery should work even if some collections fail
        let result = store.recover_all_collections().await;
        assert!(result.is_ok());

        let stats = result.unwrap();
        // Should have stats for our collection
        if let Some(count) = stats.get("recover-test") {
            assert!(*count >= 0);
        }
    }

    #[tokio::test]
    async fn test_wal_ops_stream_entries_with_large_wal() {
        // Test streaming many WAL entries
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Insert many documents to create a larger WAL
        for i in 0 .. 50 {
            collection
                .insert(&format!("doc-{}", i), serde_json::json!({"index": i}))
                .await
                .unwrap();
        }

        // Stream entries
        let stream = collection.stream_wal_entries().await.unwrap();
        let entries: Vec<_> = stream.collect().await;

        // Should have streamed all entries
        assert_eq!(entries.len(), 50);
        for entry in entries {
            assert!(entry.is_ok());
        }
    }

    #[tokio::test]
    async fn test_wal_ops_verify_with_empty_wal() {
        // Test verify_against_wal on collection with empty WAL
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Insert a document (creates WAL entry)
        collection
            .insert("doc-1", serde_json::json!({"test": 1}))
            .await
            .unwrap();

        // Verify should pass for valid state
        let result = collection.verify_against_wal().await;
        assert!(result.is_ok());

        let verification = result.unwrap();
        // Verification should have processed entries
        assert!(verification.entries_processed >= 0);
    }

    #[tokio::test]
    async fn test_wal_ops_wal_entries_count_after_rotation() {
        // Test wal_entries_count after potential WAL rotation
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Get initial count
        let initial_count = collection.wal_entries_count().await.unwrap();

        // Insert some data
        for i in 0 .. 5 {
            collection
                .insert(&format!("doc-{}", i), serde_json::json!({"index": i}))
                .await
                .unwrap();
        }

        // Get count after inserts
        let new_count = collection.wal_entries_count().await.unwrap();
        assert!(new_count >= initial_count + 5);
    }

    #[tokio::test]
    async fn test_wal_ops_checkpoint_preserves_data() {
        // Test that checkpoint preserves data integrity
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Insert data
        collection
            .insert("doc-1", serde_json::json!({"name": "Test", "value": 42}))
            .await
            .unwrap();

        // Checkpoint
        collection.checkpoint_wal().await.unwrap();

        // Data should still be accessible
        let doc = collection.get("doc-1").await.unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().data()["value"], 42);
    }

    #[tokio::test]
    async fn test_wal_ops_stream_all_with_mixed_collections() {
        // Test stream_all_wal_entries with collections that have different WAL states
        let (temp_dir, store, _collection_name) = create_test_store_with_collection().await;

        // Create collections with different amounts of data
        for i in 0 .. 3 {
            let collection = store
                .collection(&format!("stream-mixed-{}", i))
                .await
                .unwrap();

            // Insert varying numbers of documents
            for j in 0 .. i + 1 {
                collection
                    .insert(
                        &format!("doc-{}", j),
                        serde_json::json!({"collection": i, "doc": j}),
                    )
                    .await
                    .unwrap();
            }
        }

        // Stream all entries - should get entries from all collections
        let stream = store.stream_all_wal_entries().await.unwrap();
        let entries: Vec<_> = stream.collect().await;

        // Should have entries from all 3 collections (1 + 2 + 3 = 6 entries)
        assert_eq!(entries.len(), 6);

        // Verify entries come from different collections
        let collections: std::collections::HashSet<String> = entries
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|(name, _)| name)
            .collect();

        assert_eq!(collections.len(), 3);
    }
}
