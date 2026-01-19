//! WAL recovery functionality.
//!
//! This module provides recovery of collections from WAL entries.
//! Unlike the previous flawed approach, this recovery:
//! 1. Only replays operations that haven't been applied yet
//! 2. Handles conflicts gracefully
//! 3. Is idempotent (can be run multiple times safely)

use std::collections::HashMap;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::{EntryType, LogEntry, Result, WalDocumentOps, WalManager};

/// Result of WAL recovery operation
#[derive(Debug)]
pub struct WalRecoveryResult {
    /// Number of operations successfully recovered
    pub recovered_operations: usize,
    /// Operations that were skipped (already applied)
    pub skipped_operations:   usize,
    /// Operations that failed to recover
    pub failed_operations:    usize,
    /// Detailed failure reasons
    pub failures:             Vec<WalRecoveryFailure>,
}

/// Details of a recovery failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalRecoveryFailure {
    /// Transaction ID of the failed operation
    pub transaction_id: String,
    /// Document ID affected
    pub document_id:    String,
    /// Type of operation that failed
    pub operation_type: String,
    /// Reason for failure
    pub reason:         String,
}

/// Recover collection state from WAL entries
///
/// This function replays WAL entries to restore the collection to its
/// correct state. It only applies operations that haven't been applied yet
/// and handles conflicts gracefully.
pub async fn recover_from_wal_safe<D>(
    wal: &WalManager,
    document_ops: &D,
) -> Result<WalRecoveryResult>
where
    D: WalDocumentOps,
{
    let mut recovered = 0;
    let mut skipped = 0;
    let mut failed = 0;
    let mut failures = Vec::new();

    // Track applied operations to avoid duplicates
    let mut applied_operations = HashMap::new(); // (doc_id, txn_id) -> applied

    let stream = wal.stream_entries();
    let mut pinned_stream = std::pin::pin!(stream);
    while let Some(entry_result) = pinned_stream.next().await {
        match entry_result {
            Ok(entry) => {
                let key = (
                    entry.document_id_str().to_string(),
                    entry.transaction_id_str().to_string(),
                );

                // Skip if this operation was already applied
                if applied_operations.contains_key(&key) {
                    skipped += 1;
                    continue;
                }

                match replay_wal_entry_safe(&entry, document_ops).await {
                    Ok(true) => {
                        recovered += 1;
                        applied_operations.insert(key, true);
                    },
                    Ok(false) => {
                        skipped += 1;
                        applied_operations.insert(key, true);
                    },
                    Err(e) => {
                        failed += 1;
                        failures.push(WalRecoveryFailure {
                            transaction_id: entry.transaction_id_str().to_string(),
                            document_id:    entry.document_id_str().to_string(),
                            operation_type: format!("{:?}", entry.entry_type),
                            reason:         format!("{}", e),
                        });
                    },
                }
            },
            Err(e) => {
                failed += 1;
                failures.push(WalRecoveryFailure {
                    transaction_id: "unknown".to_string(),
                    document_id:    "unknown".to_string(),
                    operation_type: "read".to_string(),
                    reason:         format!("Failed to read WAL entry: {}", e),
                });
            },
        }
    }

    debug!(
        "WAL recovery completed: {} recovered, {} skipped, {} failed",
        recovered,
        skipped,
        failed
    );

    Ok(WalRecoveryResult {
        recovered_operations: recovered,
        skipped_operations: skipped,
        failed_operations: failed,
        failures,
    })
}

/// Safely replay a single WAL entry
///
/// Returns:
/// - Ok(true) if operation was applied
/// - Ok(false) if operation was skipped (already applied or conflict)
/// - Err(_) if operation failed
async fn replay_wal_entry_safe<D>(
    entry: &LogEntry,
    document_ops: &D,
) -> Result<bool>
where
    D: WalDocumentOps,
{
    match entry.entry_type {
        EntryType::Insert => {
            if let Some(data_str) = &entry.data {
                // Parse the JSON data
                let data: serde_json::Value = serde_json::from_str(data_str).map_err(|e| {
                    crate::error::WalError::Serialization(format!("Invalid JSON in WAL insert: {}", e))
                })?;

                // Check if document already exists
                match document_ops.get_document(entry.document_id_str()).await {
                    Ok(Some(_)) => {
                        // Document already exists, skip insert
                        debug!(
                            "Skipping insert for existing document {}",
                            entry.document_id_str()
                        );
                        Ok(false)
                    },
                    Ok(None) => {
                        // Document doesn't exist, apply insert
                        document_ops.apply_operation(&EntryType::Insert, entry.document_id_str(), Some(data)).await?;
                        Ok(true)
                    },
                    Err(e) => {
                        // Error checking document, fail operation
                        Err(e)
                    },
                }
            }
            else {
                warn!(
                    "WAL insert entry missing data for document {}",
                    entry.document_id_str()
                );
                Ok(false)
            }
        },
        EntryType::Update => {
            if let Some(data_str) = &entry.data {
                // Parse the JSON data
                let data: serde_json::Value = serde_json::from_str(data_str).map_err(|e| {
                    crate::error::WalError::Serialization(format!("Invalid JSON in WAL update: {}", e))
                })?;

                // Check if document exists
                match document_ops.get_document(entry.document_id_str()).await {
                    Ok(Some(existing_doc)) => {
                        // Document exists, check if update is needed
                        if existing_doc != data {
                            document_ops.apply_operation(&EntryType::Update, entry.document_id_str(), Some(data)).await?;
                            Ok(true)
                        }
                        else {
                            Ok(false)
                        }
                    },
                    Ok(None) => {
                        // Document doesn't exist, this is an error for update
                        warn!(
                            "Skipping update for non-existent document {}",
                            entry.document_id_str()
                        );
                        Ok(false)
                    },
                    Err(e) => {
                        // Error checking document, fail operation
                        Err(e)
                    },
                }
            }
            else {
                warn!(
                    "WAL update entry missing data for document {}",
                    entry.document_id_str()
                );
                Ok(false)
            }
        },
        EntryType::Delete => {
            // Check if document exists
            match document_ops.get_document(entry.document_id_str()).await {
                Ok(Some(_)) => {
                    // Document exists, apply delete
                    document_ops.apply_operation(&EntryType::Delete, entry.document_id_str(), None).await?;
                    Ok(true)
                },
                Ok(None) => {
                    // Document doesn't exist, skip delete
                    debug!(
                        "Skipping delete for non-existent document {}",
                        entry.document_id_str()
                    );
                    Ok(false)
                },
                Err(e) => {
                    // Error checking document, fail operation
                    Err(e)
                },
            }
        },
        // Transaction control entries don't affect document state
        EntryType::Begin | EntryType::Commit | EntryType::Rollback => Ok(false),
    }
}

/// Recover collection from WAL with conflict resolution
///
/// This is a more aggressive recovery that attempts to resolve conflicts
/// by overwriting conflicting states.
pub async fn recover_from_wal_force<D>(
    wal: &WalManager,
    document_ops: &D,
) -> Result<WalRecoveryResult>
where
    D: WalDocumentOps,
{
    let mut recovered = 0;
    let mut skipped = 0;
    let mut failed = 0;
    let mut failures = Vec::new();

    let stream = wal.stream_entries();
    let mut pinned_stream = std::pin::pin!(stream);
    while let Some(entry_result) = pinned_stream.next().await {
        match entry_result {
            Ok(entry) => {
                match replay_wal_entry_force(&entry, document_ops).await {
                    Ok(applied) => {
                        if applied {
                            recovered += 1;
                        } else {
                            skipped += 1;
                        }
                    },
                    Err(e) => {
                        failed += 1;
                        failures.push(WalRecoveryFailure {
                            transaction_id: entry.transaction_id_str().to_string(),
                            document_id:    entry.document_id_str().to_string(),
                            operation_type: format!("{:?}", entry.entry_type),
                            reason:         format!("{}", e),
                        });
                    },
                }
            },
            Err(e) => {
                failed += 1;
                failures.push(WalRecoveryFailure {
                    transaction_id: "unknown".to_string(),
                    document_id:    "unknown".to_string(),
                    operation_type: "read".to_string(),
                    reason:         format!("Failed to read WAL entry: {}", e),
                });
            },
        }
    }

    debug!(
        "Forced WAL recovery completed: {} recovered, {} skipped, {} failed",
        recovered,
        skipped,
        failed
    );

    Ok(WalRecoveryResult {
        recovered_operations: recovered,
        skipped_operations: skipped,
        failed_operations: failed,
        failures,
    })
}

/// Force replay a WAL entry (overwrites conflicts)
async fn replay_wal_entry_force<D>(
    entry: &LogEntry,
    document_ops: &D,
) -> Result<bool>
where
    D: WalDocumentOps,
{
    match entry.entry_type {
        EntryType::Insert | EntryType::Update => {
            if let Some(data_str) = &entry.data {
                let data: serde_json::Value = serde_json::from_str(data_str).map_err(|e| {
                    crate::error::WalError::Serialization(format!("Invalid JSON in WAL entry: {}", e))
                })?;

                // For force recovery, always apply the operation
                document_ops.apply_operation(&entry.entry_type, entry.document_id_str(), Some(data)).await?;
                Ok(true)
            }
            else {
                Ok(false)
            }
        },
        EntryType::Delete => {
            // Force delete (ignore if document doesn't exist)
            match document_ops.apply_operation(&EntryType::Delete, entry.document_id_str(), None).await {
                Ok(_) => Ok(true),
                Err(crate::error::WalError::Io { .. }) => Ok(false), // Assume not found
                Err(e) => Err(e),
            }
        },
        // Transaction control entries don't affect document state
        EntryType::Begin | EntryType::Commit | EntryType::Rollback => Ok(false),
    }
}