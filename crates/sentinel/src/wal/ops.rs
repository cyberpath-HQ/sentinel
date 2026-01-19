//! Core WAL operations and trait definitions.

use std::{collections::HashMap, pin::Pin};

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use tracing::{debug, error};
use sentinel_wal::LogEntry;

use crate::{Collection, Result, Store};
use super::{recovery::WalRecoveryResult, verification::WalVerificationResult};

/// Extension trait for Store to add WAL operations
#[async_trait]
pub trait StoreWalOps {
    /// Perform a checkpoint on all collections
    async fn checkpoint_all_collections(&self) -> Result<()>;

    /// Stream WAL entries from all collections
    async fn stream_all_wal_entries(&self) -> Result<Pin<Box<dyn Stream<Item = Result<(String, LogEntry)>> + Send>>>;

    /// Verify all collections against their WAL files
    async fn verify_all_collections(&self) -> Result<HashMap<String, Vec<super::verification::WalVerificationIssue>>>;

    /// Recover all collections from their WAL files
    async fn recover_all_collections(&self) -> Result<HashMap<String, usize>>;
}

/// Extension trait for Collection to add WAL operations
#[async_trait]
pub trait CollectionWalOps {
    /// Perform a checkpoint on this collection's WAL
    async fn checkpoint_wal(&self) -> Result<()>;

    /// Stream WAL entries for this collection
    async fn stream_wal_entries(&self) -> Result<Pin<Box<dyn Stream<Item = Result<LogEntry>> + Send>>>;

    /// Verify this collection against its WAL file
    async fn verify_against_wal(&self) -> Result<WalVerificationResult>;

    /// Recover this collection from its WAL file
    async fn recover_from_wal(&self) -> Result<WalRecoveryResult>;

    /// Get the current WAL size in bytes
    async fn wal_size(&self) -> Result<u64>;

    /// Get the number of entries in the WAL
    async fn wal_entries_count(&self) -> Result<usize>;
}

#[async_trait]
impl StoreWalOps for Store {
    async fn checkpoint_all_collections(&self) -> Result<()> {
        let collections = self.list_collections().await?;
        for collection_name in collections {
            let collection = self.collection(&collection_name).await?;
            collection.checkpoint_wal().await?;
        }

        debug!("Checkpoint completed for all collections");
        Ok(())
    }

    async fn stream_all_wal_entries(&self) -> Result<Pin<Box<dyn Stream<Item = Result<(String, LogEntry)>> + Send>>> {
        let collections = self.list_collections().await?;
        let mut streams = Vec::new();

        for collection_name in collections {
            let collection = self.collection(&collection_name).await?;
            if let Ok(stream) = CollectionWalOps::stream_wal_entries(&collection).await {
                let collection_name_clone = collection_name.clone();
                let mapped_stream =
                    stream.map(move |entry: Result<LogEntry>| entry.map(|e| (collection_name_clone.clone(), e)));
                streams.push(Box::pin(mapped_stream));
            }
        }

        Ok(Box::pin(futures::stream::select_all(streams)))
    }

    async fn verify_all_collections(&self) -> Result<HashMap<String, Vec<super::verification::WalVerificationIssue>>> {
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
                        vec![super::verification::WalVerificationIssue {
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

    async fn recover_all_collections(&self) -> Result<HashMap<String, usize>> {
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
    async fn checkpoint_wal(&self) -> Result<()> {
        if let Some(wal) = &self.wal_manager {
            wal.checkpoint().await?;
            debug!("WAL checkpoint completed for collection {}", self.name());
        }
        Ok(())
    }

    async fn stream_wal_entries(&self) -> Result<Pin<Box<dyn Stream<Item = Result<LogEntry>> + Send>>> {
        if let Some(wal) = &self.wal_manager {
            let entries = wal.read_all_entries().await?;
            let stream = futures::stream::iter(entries.into_iter().map(Ok));
            Ok(Box::pin(stream))
        } else {
            Ok(Box::pin(futures::stream::empty()))
        }
    }

    async fn verify_against_wal(&self) -> Result<WalVerificationResult> { self.verify_wal_consistency().await }

    async fn recover_from_wal(&self) -> Result<WalRecoveryResult> { self.recover_from_wal_safe().await }

    async fn wal_size(&self) -> Result<u64> {
        if let Some(wal) = &self.wal_manager {
            Ok(wal.size().await?)
        }
        else {
            Ok(0)
        }
    }

    async fn wal_entries_count(&self) -> Result<usize> {
        if let Some(wal) = &self.wal_manager {
            let mut count = 0;
            let stream = wal.stream_entries();
            use futures::StreamExt;
            futures::pin_mut!(stream);
            while let Some(result) = stream.next().await {
                result?; // Check for errors but don't need the entry
                count += 1;
            }
            Ok(count)
        } else {
            Ok(0)
        }
    }
}
