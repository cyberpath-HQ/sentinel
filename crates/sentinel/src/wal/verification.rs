//! WAL verification functionality.
//!
//! This module provides verification of WAL consistency and data integrity.
//! Unlike the previous flawed approach, this verifies:
//! 1. WAL internal consistency (operations are valid sequences)
//! 2. Final WAL state matches current disk state
//! 3. No corrupted or invalid entries exist

use std::collections::HashMap;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sentinel_wal::{EntryType, LogEntry};

use crate::{Collection, Result};

/// Issues found during WAL verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalVerificationIssue {
    /// Transaction ID where the issue occurred
    pub transaction_id: String,
    /// Document ID affected
    pub document_id:    String,
    /// Description of the issue
    pub description:    String,
    /// Whether this is a critical issue
    pub is_critical:    bool,
}

/// Result of WAL verification
#[derive(Debug)]
pub struct WalVerificationResult {
    /// Issues found during verification
    pub issues:           Vec<WalVerificationIssue>,
    /// Whether verification passed (no critical issues)
    pub passed:           bool,
    /// Final document states according to WAL
    pub wal_final_states: HashMap<String, serde_json::Value>,
}

impl Collection {
    /// Verify WAL consistency and final state against disk
    ///
    /// This method:
    /// 1. Replays all WAL entries to compute final expected states
    /// 2. Compares final WAL states with actual disk states
    /// 3. Checks for WAL internal consistency
    pub async fn verify_wal_consistency(&self) -> Result<WalVerificationResult> {
        let mut issues = Vec::new();
        let mut wal_states = HashMap::new(); // document_id -> final_data
        let mut active_transactions = HashMap::new(); // txn_id -> operations

        if let Some(wal) = &self.wal_manager {
            let stream = wal.stream_entries();
            let mut pinned_stream = std::pin::pin!(stream);
            while let Some(entry_result) = pinned_stream.next().await {
                match entry_result {
                    Ok(entry) => {
                        if let Some(issue) = self
                            .verify_wal_entry_consistency(&entry, &mut wal_states, &mut active_transactions)
                            .await?
                        {
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

        // Check that final WAL states match disk states
        for (doc_id, wal_data) in &wal_states {
            match self.get(doc_id).await {
                Ok(Some(doc)) => {
                    if doc.data() != wal_data {
                        issues.push(WalVerificationIssue {
                            transaction_id: "final_check".to_string(),
                            document_id:    doc_id.clone(),
                            description:    format!(
                                "Final WAL state doesn't match disk state for document {}",
                                doc_id
                            ),
                            is_critical:    true,
                        });
                    }
                },
                Ok(None) => {
                    issues.push(WalVerificationIssue {
                        transaction_id: "final_check".to_string(),
                        document_id:    doc_id.clone(),
                        description:    format!("Document {} exists in WAL but not on disk", doc_id),
                        is_critical:    true,
                    });
                },
                Err(e) => {
                    issues.push(WalVerificationIssue {
                        transaction_id: "final_check".to_string(),
                        document_id:    doc_id.clone(),
                        description:    format!("Failed to read document {} from disk: {}", doc_id, e),
                        is_critical:    true,
                    });
                },
            }
        }

        let passed = !issues.iter().any(|issue| issue.is_critical);

        Ok(WalVerificationResult {
            issues,
            passed,
            wal_final_states: wal_states,
        })
    }

    /// Verify a single WAL entry for consistency
    async fn verify_wal_entry_consistency(
        &self,
        entry: &LogEntry,
        wal_states: &mut HashMap<String, serde_json::Value>,
        active_transactions: &mut HashMap<String, Vec<LogEntry>>,
    ) -> Result<Option<WalVerificationIssue>> {
        let txn_id = entry.transaction_id_str();
        let doc_id = entry.document_id_str();

        // Track transaction operations
        active_transactions
            .entry(txn_id.to_string())
            .or_insert_with(Vec::new)
            .push(entry.clone());

        match entry.entry_type {
            EntryType::Begin => {
                // Transaction begin - should not have data
                if entry.data.is_some() {
                    return Ok(Some(WalVerificationIssue {
                        transaction_id: txn_id.to_string(),
                        document_id:    doc_id.to_string(),
                        description:    "Transaction begin entry should not contain data".to_string(),
                        is_critical:    false,
                    }));
                }
            },
            EntryType::Insert => {
                if let Some(data_str) = &entry.data {
                    match serde_json::from_str(data_str) {
                        Ok(data) => {
                            // Check if document already exists in WAL state
                            if wal_states.contains_key(doc_id) {
                                return Ok(Some(WalVerificationIssue {
                                    transaction_id: txn_id.to_string(),
                                    document_id:    doc_id.to_string(),
                                    description:    format!("Insert operation for existing document {}", doc_id),
                                    is_critical:    true,
                                }));
                            }
                            wal_states.insert(doc_id.to_string(), data);
                        },
                        Err(e) => {
                            return Ok(Some(WalVerificationIssue {
                                transaction_id: txn_id.to_string(),
                                document_id:    doc_id.to_string(),
                                description:    format!("Invalid JSON data in insert operation: {}", e),
                                is_critical:    true,
                            }));
                        },
                    }
                }
                else {
                    return Ok(Some(WalVerificationIssue {
                        transaction_id: txn_id.to_string(),
                        document_id:    doc_id.to_string(),
                        description:    "Insert operation missing data".to_string(),
                        is_critical:    true,
                    }));
                }
            },
            EntryType::Update => {
                if let Some(data_str) = &entry.data {
                    match serde_json::from_str(data_str) {
                        Ok(data) => {
                            // Check if document exists in WAL state
                            if !wal_states.contains_key(doc_id) {
                                return Ok(Some(WalVerificationIssue {
                                    transaction_id: txn_id.to_string(),
                                    document_id:    doc_id.to_string(),
                                    description:    format!("Update operation for non-existent document {}", doc_id),
                                    is_critical:    true,
                                }));
                            }
                            wal_states.insert(doc_id.to_string(), data);
                        },
                        Err(e) => {
                            return Ok(Some(WalVerificationIssue {
                                transaction_id: txn_id.to_string(),
                                document_id:    doc_id.to_string(),
                                description:    format!("Invalid JSON data in update operation: {}", e),
                                is_critical:    true,
                            }));
                        },
                    }
                }
                else {
                    return Ok(Some(WalVerificationIssue {
                        transaction_id: txn_id.to_string(),
                        document_id:    doc_id.to_string(),
                        description:    "Update operation missing data".to_string(),
                        is_critical:    true,
                    }));
                }
            },
            EntryType::Delete => {
                // Check if document exists in WAL state
                if !wal_states.contains_key(doc_id) {
                    return Ok(Some(WalVerificationIssue {
                        transaction_id: txn_id.to_string(),
                        document_id:    doc_id.to_string(),
                        description:    format!("Delete operation for non-existent document {}", doc_id),
                        is_critical:    false, // Not critical - might be double delete
                    }));
                }
                wal_states.remove(doc_id);
            },
            EntryType::Commit => {
                // Transaction commit - validate the transaction
                if let Some(ops) = active_transactions.get(txn_id) {
                    if let Some(issue) = self.verify_transaction_consistency(ops).await? {
                        return Ok(Some(issue));
                    }
                }
                active_transactions.remove(txn_id);
            },
            EntryType::Rollback => {
                // Transaction rollback - undo all operations in this transaction
                if let Some(ops) = active_transactions.remove(txn_id) {
                    for op in ops.iter().rev() {
                        match op.entry_type {
                            EntryType::Insert => {
                                wal_states.remove(op.document_id_str());
                            },
                            EntryType::Update => {
                                // For rollback, we'd need to track previous states
                                // For now, mark as issue since we can't reliably rollback
                                return Ok(Some(WalVerificationIssue {
                                    transaction_id: txn_id.to_string(),
                                    document_id:    op.document_id_str().to_string(),
                                    description:    "Transaction rollback not fully supported in verification"
                                        .to_string(),
                                    is_critical:    false,
                                }));
                            },
                            EntryType::Delete => {
                                // Restore would require knowing previous state
                                return Ok(Some(WalVerificationIssue {
                                    transaction_id: txn_id.to_string(),
                                    document_id:    op.document_id_str().to_string(),
                                    description:    "Transaction rollback not fully supported in verification"
                                        .to_string(),
                                    is_critical:    false,
                                }));
                            },
                            _ => {}, // Begin/Commit/Rollback don't affect state
                        }
                    }
                }
            },
        }

        Ok(None)
    }

    /// Verify transaction consistency
    async fn verify_transaction_consistency(&self, ops: &[LogEntry]) -> Result<Option<WalVerificationIssue>> {
        // Check that transaction has proper begin/commit structure
        let has_begin = ops.iter().any(|op| op.entry_type == EntryType::Begin);
        let has_commit = ops.iter().any(|op| op.entry_type == EntryType::Commit);

        if !has_begin {
            return Ok(Some(WalVerificationIssue {
                transaction_id: ops[0].transaction_id_str().to_string(),
                document_id:    "transaction".to_string(),
                description:    "Transaction missing begin entry".to_string(),
                is_critical:    false,
            }));
        }

        if !has_commit {
            return Ok(Some(WalVerificationIssue {
                transaction_id: ops[0].transaction_id_str().to_string(),
                document_id:    "transaction".to_string(),
                description:    "Transaction missing commit entry".to_string(),
                is_critical:    false,
            }));
        }

        Ok(None)
    }
}
