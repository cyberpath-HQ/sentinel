//! Core WAL operations and trait definitions.

use std::{collections::HashMap, pin::Pin};

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use tracing::{debug, error};
use sentinel_wal::{
    recover_from_wal_safe,
    verify_wal_consistency,
    LogEntry,
    WalRecoveryResult,
    WalVerificationIssue,
    WalVerificationResult,
};

use crate::{Collection, Store};

/// Extension trait for Store to add WAL operations
#[async_trait]
pub trait StoreWalOps {
    /// Perform a checkpoint on all collections
    async fn checkpoint_all_collections(&self) -> crate::Result<()>;

    /// Stream WAL entries from all collections
    async fn stream_all_wal_entries(
        &self,
    ) -> crate::Result<Pin<Box<dyn Stream<Item = crate::Result<(String, LogEntry)>> + Send>>>;

    /// Verify all collections against their WAL files
    async fn verify_all_collections(&self) -> crate::Result<HashMap<String, Vec<WalVerificationIssue>>>;

    /// Recover all collections from their WAL files
    async fn recover_all_collections(&self) -> crate::Result<HashMap<String, usize>>;
}

/// Extension trait for Collection to add WAL operations
#[async_trait]
pub trait CollectionWalOps {
    /// Perform a checkpoint on this collection's WAL
    async fn checkpoint_wal(&self) -> crate::Result<()>;

    /// Stream WAL entries for this collection
    async fn stream_wal_entries(&self) -> crate::Result<Pin<Box<dyn Stream<Item = crate::Result<LogEntry>> + Send>>>;

    /// Verify this collection against its WAL file
    async fn verify_against_wal(&self) -> crate::Result<WalVerificationResult>;

    /// Recover this collection from its WAL file
    async fn recover_from_wal(&self) -> crate::Result<WalRecoveryResult>;

    /// Get the current WAL size in bytes
    async fn wal_size(&self) -> crate::Result<u64>;

    /// Get the number of entries in the WAL
    async fn wal_entries_count(&self) -> crate::Result<usize>;
}

#[async_trait]
impl StoreWalOps for Store {
    async fn checkpoint_all_collections(&self) -> crate::Result<()> {
        let collections = self.list_collections().await?;
        for collection_name in collections {
            let collection = self.collection(&collection_name).await?;
            CollectionWalOps::checkpoint_wal(&collection).await?;
        }

        debug!("Checkpoint completed for all collections");
        Ok(())
    }

    async fn stream_all_wal_entries(
        &self,
    ) -> crate::Result<Pin<Box<dyn Stream<Item = crate::Result<(String, LogEntry)>> + Send>>> {
        let collections = self.list_collections().await?;
        let mut streams = Vec::new();

        for collection_name in collections {
            let collection = self.collection(&collection_name).await?;
            if let Ok(stream) = CollectionWalOps::stream_wal_entries(&collection).await {
                let collection_name_clone = collection_name.clone();
                let mapped_stream =
                    stream.map(move |entry: crate::Result<LogEntry>| entry.map(|e| (collection_name_clone.clone(), e)));
                streams.push(Box::pin(mapped_stream));
            }
        }

        Ok(Box::pin(futures::stream::select_all(streams)))
    }

    async fn verify_all_collections(&self) -> crate::Result<HashMap<String, Vec<WalVerificationIssue>>> {
        let collections = self.list_collections().await?;
        let mut results = HashMap::new();

        for collection_name in collections {
            let collection = self.collection(&collection_name).await?;
            match CollectionWalOps::verify_against_wal(&collection).await {
                Ok(verification_result) => {
                    if !verification_result.issues.is_empty() {
                        results.insert(collection_name, verification_result.issues);
                    }
                },
                Err(e) => {
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
            }
        }

        Ok(results)
    }

    async fn recover_all_collections(&self) -> crate::Result<HashMap<String, usize>> {
        let collections = self.list_collections().await?;
        let mut results = HashMap::new();

        for collection_name in collections {
            let collection = self.collection(&collection_name).await?;
            match CollectionWalOps::recover_from_wal(&collection).await {
                Ok(recovery_result) => {
                    results.insert(collection_name, recovery_result.recovered_operations);
                },
                Err(e) => {
                    error!("Failed to recover collection {}: {}", collection_name, e);
                    return Err(e);
                },
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl CollectionWalOps for Collection {
    async fn checkpoint_wal(&self) -> crate::Result<()> {
        if let Some(wal) = &self.wal_manager {
            wal.checkpoint().await?;
            debug!("WAL checkpoint completed for collection {}", self.name());
        }
        Ok(())
    }

    async fn stream_wal_entries(&self) -> crate::Result<Pin<Box<dyn Stream<Item = crate::Result<LogEntry>> + Send>>> {
        if let Some(wal) = &self.wal_manager {
            let entries = wal.read_all_entries().await?;
            let stream = futures::stream::iter(entries.into_iter().map(Ok));
            Ok(Box::pin(stream))
        }
        else {
            Ok(Box::pin(futures::stream::empty()))
        }
    }

    async fn verify_against_wal(&self) -> crate::Result<WalVerificationResult> {
        if let Some(wal) = &self.wal_manager {
            verify_wal_consistency(wal, self).await.map_err(Into::into)
        }
        else {
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
            recover_from_wal_safe(wal, self).await.map_err(Into::into)
        }
        else {
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
            Ok(wal.size().await?)
        }
        else {
            Ok(0)
        }
    }

    async fn wal_entries_count(&self) -> crate::Result<usize> {
        if let Some(wal) = &self.wal_manager {
            Ok(wal.entries_count().await?)
        }
        else {
            Ok(0)
        }
    }
}
