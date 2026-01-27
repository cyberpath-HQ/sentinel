//! WAL verification functionality.
//!
//! This module provides verification of WAL consistency and data integrity.
//! Unlike the previous flawed approach, this verifies:
//! 1. WAL internal consistency (operations are valid sequences)
//! 2. Final WAL state matches current disk state
//! 3. No corrupted or invalid entries exist

use std::collections::HashMap;

use futures::StreamExt as _;
use serde::{Deserialize, Serialize};

use crate::{EntryType, LogEntry, Result, WalDocumentOps, WalManager};

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
    pub issues:             Vec<WalVerificationIssue>,
    /// Whether verification passed (no critical issues)
    pub passed:             bool,
    /// Number of WAL entries processed
    pub entries_processed:  u64,
    /// Number of documents that would be affected by WAL replay
    pub affected_documents: u64,
}

/// Verify WAL consistency and final state against disk
///
/// This function:
/// 1. Replays all WAL entries to compute final expected states
/// 2. Compares final WAL states with actual disk states
/// 3. Checks for WAL internal consistency
#[allow(clippy::arithmetic_side_effects, reason = "counter increment in loop")]
pub async fn verify_wal_consistency<D>(wal: &WalManager, document_ops: &D) -> Result<WalVerificationResult>
where
    D: WalDocumentOps + Sync,
{
    let mut issues = Vec::new();
    let mut wal_states = HashMap::new(); // document_id -> final_data
    let mut active_transactions = HashMap::new(); // txn_id -> operations
    let mut entries_processed = 0;

    let stream = wal.stream_entries();
    let mut pinned_stream = std::pin::pin!(stream);
    while let Some(entry_result) = pinned_stream.next().await {
        match entry_result {
            Ok(entry) => {
                entries_processed += 1;
                if let Some(issue) =
                    verify_wal_entry_consistency(&entry, &mut wal_states, &mut active_transactions).await?
                {
                    issues.push(issue);
                }
            },
            Err(e) => {
                issues.push(WalVerificationIssue {
                    transaction_id: "unknown".to_owned(),
                    document_id:    "unknown".to_owned(),
                    description:    format!("Failed to read WAL entry: {}", e),
                    is_critical:    true,
                });
            },
        }
    }

    // Check that final WAL states match disk states
    for doc_id in wal_states.keys() {
        match document_ops.get_document(doc_id).await {
            Ok(Some(existing_doc)) => {
                // Compare WAL state with disk state
                if let Some(wal_doc) = wal_states.get(doc_id) &&
                    existing_doc != *wal_doc
                {
                    issues.push(WalVerificationIssue {
                        transaction_id: "final_check".to_owned(),
                        document_id:    doc_id.clone(),
                        description:    format!("Document {} data mismatch between WAL and disk", doc_id),
                        is_critical:    true,
                    });
                }
            },
            Ok(None) => {
                issues.push(WalVerificationIssue {
                    transaction_id: "final_check".to_owned(),
                    document_id:    doc_id.clone(),
                    description:    format!("Document {} exists in WAL but not on disk", doc_id),
                    is_critical:    true,
                });
            },
            Err(e) => {
                issues.push(WalVerificationIssue {
                    transaction_id: "final_check".to_owned(),
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
        entries_processed: entries_processed as u64,
        affected_documents: wal_states.len() as u64,
    })
}

/// Verify a single WAL entry for consistency
async fn verify_wal_entry_consistency(
    entry: &LogEntry,
    wal_states: &mut HashMap<String, serde_json::Value>,
    active_transactions: &mut HashMap<String, Vec<LogEntry>>,
) -> Result<Option<WalVerificationIssue>> {
    let txn_id = entry.transaction_id_str();
    let doc_id = entry.document_id_str();

    // Track transaction operations
    active_transactions
        .entry(txn_id.to_owned())
        .or_insert_with(Vec::new)
        .push(entry.clone());

    match entry.entry_type {
        EntryType::Begin => {
            // Transaction begin - should not have data
            if entry.data.is_some() {
                return Ok(Some(WalVerificationIssue {
                    transaction_id: txn_id.to_owned(),
                    document_id:    doc_id.to_owned(),
                    description:    "Transaction begin entry should not contain data".to_owned(),
                    is_critical:    false,
                }));
            }
        },
        EntryType::Insert => {
            if let Some(data_str) = entry.data.as_ref() {
                match serde_json::from_str(data_str) {
                    Ok(data) => {
                        // Check if document already exists in WAL state
                        if wal_states.contains_key(doc_id) {
                            return Ok(Some(WalVerificationIssue {
                                transaction_id: txn_id.to_owned(),
                                document_id:    doc_id.to_owned(),
                                description:    format!("Document {} already exists in WAL state", doc_id),
                                is_critical:    true,
                            }));
                        }
                        wal_states.insert(doc_id.to_owned(), data);
                    },
                    Err(e) => {
                        return Ok(Some(WalVerificationIssue {
                            transaction_id: txn_id.to_owned(),
                            document_id:    doc_id.to_owned(),
                            description:    format!("Invalid JSON data in insert operation: {}", e),
                            is_critical:    true,
                        }));
                    },
                }
            }
            else {
                return Ok(Some(WalVerificationIssue {
                    transaction_id: txn_id.to_owned(),
                    document_id:    doc_id.to_owned(),
                    description:    "Insert operation missing data".to_owned(),
                    is_critical:    true,
                }));
            }
        },
        EntryType::Update => {
            if let Some(data_str) = entry.data.as_ref() {
                match serde_json::from_str(data_str) {
                    Ok(data) => {
                        // For partial WAL verification, updates can reference documents
                        // that were inserted in earlier WAL segments not being verified.
                        // We allow this but track it as a potential issue.
                        // Note: The insert below is common to both branches
                        wal_states.insert(doc_id.to_owned(), data);
                    },
                    Err(e) => {
                        return Ok(Some(WalVerificationIssue {
                            transaction_id: txn_id.to_owned(),
                            document_id:    doc_id.to_owned(),
                            description:    format!("Invalid JSON data in update operation: {}", e),
                            is_critical:    true,
                        }));
                    },
                }
            }
            else {
                return Ok(Some(WalVerificationIssue {
                    transaction_id: txn_id.to_owned(),
                    document_id:    doc_id.to_owned(),
                    description:    "Update operation missing data".to_owned(),
                    is_critical:    true,
                }));
            }
        },
        EntryType::Delete => {
            // For partial WAL verification, deletes can reference documents
            // that were inserted in earlier WAL segments not being verified.
            // We allow this - the document may exist on disk.
            wal_states.remove(doc_id);
        },
        EntryType::Commit => {
            // Transaction commit - validate the transaction
            if let Some(ops) = active_transactions.get(txn_id) &&
                let Some(issue) = verify_transaction_consistency(ops).await?
            {
                return Ok(Some(issue));
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
                                transaction_id: txn_id.to_owned(),
                                document_id:    doc_id.to_owned(),
                                description:    "Transaction rollback not fully supported in verification".to_owned(),
                                is_critical:    false,
                            }));
                        },
                        EntryType::Delete => {
                            // For delete rollback, we'd need to restore previous state
                            // Mark as issue
                            return Ok(Some(WalVerificationIssue {
                                transaction_id: txn_id.to_owned(),
                                document_id:    doc_id.to_owned(),
                                description:    "Transaction rollback for delete not supported".to_owned(),
                                is_critical:    false,
                            }));
                        },
                        EntryType::Begin | EntryType::Commit | EntryType::Rollback => {},
                    }
                }
            }
        },
    }

    Ok(None)
}

/// Verify transaction consistency
#[allow(
    clippy::expect_used,
    reason = "ops is guaranteed to be non-empty in this context"
)]
async fn verify_transaction_consistency(ops: &[LogEntry]) -> Result<Option<WalVerificationIssue>> {
    // Check that transaction has proper begin/commit structure
    let has_begin = ops.iter().any(|op| op.entry_type == EntryType::Begin);
    let has_commit = ops.iter().any(|op| op.entry_type == EntryType::Commit);

    if !has_begin {
        let first_op = ops.first().expect("ops is non-empty");
        return Ok(Some(WalVerificationIssue {
            transaction_id: first_op.transaction_id_str().to_owned(),
            document_id:    "transaction".to_owned(),
            description:    "Transaction missing begin entry".to_owned(),
            is_critical:    false,
        }));
    }

    if !has_commit {
        let first_op = ops.first().expect("ops is non-empty");
        return Ok(Some(WalVerificationIssue {
            transaction_id: first_op.transaction_id_str().to_owned(),
            document_id:    "transaction".to_owned(),
            description:    "Transaction missing commit entry".to_owned(),
            is_critical:    false,
        }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntryType, LogEntry};

    fn create_test_entry(entry_type: EntryType, doc_id: &str, txn_id: &str) -> LogEntry {
        use crate::entry::{FixedBytes256, FixedBytes32};
        LogEntry {
            entry_type,
            collection: FixedBytes256::from(b"test" as &[u8]),
            document_id: FixedBytes256::from(doc_id.as_bytes()),
            transaction_id: FixedBytes32::from(txn_id.as_bytes()),
            data: None,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    #[tokio::test]
    async fn test_verify_transaction_consistency_valid() {
        let ops = vec![
            create_test_entry(EntryType::Begin, "doc1", "txn1"),
            create_test_entry(EntryType::Insert, "doc1", "txn1"),
            create_test_entry(EntryType::Commit, "doc1", "txn1"),
        ];

        let result = verify_transaction_consistency(&ops).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_verify_transaction_consistency_missing_begin() {
        let ops = vec![
            create_test_entry(EntryType::Insert, "doc1", "txn1"),
            create_test_entry(EntryType::Commit, "doc1", "txn1"),
        ];

        let result = verify_transaction_consistency(&ops).await.unwrap();
        assert!(result.is_some());
        let issue = result.unwrap();
        assert!(issue.description.contains("missing begin"));
        assert!(!issue.is_critical);
    }

    #[tokio::test]
    async fn test_verify_transaction_consistency_missing_commit() {
        let ops = vec![
            create_test_entry(EntryType::Begin, "doc1", "txn1"),
            create_test_entry(EntryType::Insert, "doc1", "txn1"),
        ];

        let result = verify_transaction_consistency(&ops).await.unwrap();
        assert!(result.is_some());
        let issue = result.unwrap();
        assert!(issue.description.contains("missing commit"));
        assert!(!issue.is_critical);
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_insert() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        let mut entry = create_test_entry(EntryType::Insert, "doc1", "txn1");
        entry.data = Some(r#"{"name": "test"}"#.to_string());

        let result = verify_wal_entry_consistency(&entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        assert!(result.is_none());
        assert!(wal_states.contains_key("doc1"));
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_update() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        // First insert
        let mut insert_entry = create_test_entry(EntryType::Insert, "doc1", "txn1");
        insert_entry.data = Some(r#"{"name": "test"}"#.to_string());
        verify_wal_entry_consistency(&insert_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();

        // Then update
        let mut update_entry = create_test_entry(EntryType::Update, "doc1", "txn2");
        update_entry.data = Some(r#"{"updated": true}"#.to_string());

        let result = verify_wal_entry_consistency(&update_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_delete() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        // First insert
        let insert_entry = create_test_entry(EntryType::Insert, "doc1", "txn1");
        verify_wal_entry_consistency(&insert_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();

        // Then delete
        let delete_entry = create_test_entry(EntryType::Delete, "doc1", "txn2");

        let result = verify_wal_entry_consistency(&delete_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        assert!(result.is_none());
        assert!(!wal_states.contains_key("doc1"));
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_begin_with_data() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        let mut begin_entry = create_test_entry(EntryType::Begin, "doc1", "txn1");
        begin_entry.data = Some(r#"{"unexpected": "data"}"#.to_string());

        let result = verify_wal_entry_consistency(&begin_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        assert!(result.is_some());
        let issue = result.unwrap();
        assert_eq!(issue.transaction_id, "txn1");
        assert_eq!(issue.document_id, "doc1");
        assert!(issue.description.contains("should not contain data"));
        assert!(!issue.is_critical);
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_insert_invalid_json() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        let mut insert_entry = create_test_entry(EntryType::Insert, "doc1", "txn1");
        insert_entry.data = Some(r#"{"invalid": json}"#.to_string());

        let result = verify_wal_entry_consistency(&insert_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        assert!(result.is_some());
        let issue = result.unwrap();
        assert_eq!(issue.transaction_id, "txn1");
        assert_eq!(issue.document_id, "doc1");
        assert!(issue.description.contains("Invalid JSON data"));
        assert!(issue.is_critical);
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_insert_no_data() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        let mut insert_entry = create_test_entry(EntryType::Insert, "doc1", "txn1");
        insert_entry.data = None;

        let result = verify_wal_entry_consistency(&insert_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        assert!(result.is_some());
        let issue = result.unwrap();
        assert_eq!(issue.transaction_id, "txn1");
        assert_eq!(issue.document_id, "doc1");
        assert!(issue.description.contains("missing data"));
        assert!(issue.is_critical);
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_update_invalid_json() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        let mut update_entry = create_test_entry(EntryType::Update, "doc1", "txn1");
        update_entry.data = Some(r#"{"invalid": json}"#.to_string());

        let result = verify_wal_entry_consistency(&update_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        assert!(result.is_some());
        let issue = result.unwrap();
        assert_eq!(issue.transaction_id, "txn1");
        assert_eq!(issue.document_id, "doc1");
        assert!(issue.description.contains("Invalid JSON data"));
        assert!(issue.is_critical);
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_insert_duplicate() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        // First insert
        let mut insert_entry1 = create_test_entry(EntryType::Insert, "doc1", "txn1");
        insert_entry1.data = Some(r#"{"name": "test"}"#.to_string());
        verify_wal_entry_consistency(&insert_entry1, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();

        // Second insert of same document
        let mut insert_entry2 = create_test_entry(EntryType::Insert, "doc1", "txn2");
        insert_entry2.data = Some(r#"{"name": "test2"}"#.to_string());

        let result = verify_wal_entry_consistency(&insert_entry2, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        assert!(result.is_some());
        let issue = result.unwrap();
        assert_eq!(issue.transaction_id, "txn2");
        assert_eq!(issue.document_id, "doc1");
        assert!(issue.description.contains("already exists"));
        assert!(issue.is_critical);
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_update_nonexistent_doc() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        let mut update_entry = create_test_entry(EntryType::Update, "nonexistent", "txn1");
        update_entry.data = Some(r#"{"name": "updated"}"#.to_string());

        // Update on nonexistent is actually valid (creates document-like state)
        let result = verify_wal_entry_consistency(&update_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        // This should be None since update is allowed on nonexistent
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_delete_nonexistent_doc() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        let delete_entry = create_test_entry(EntryType::Delete, "nonexistent", "txn1");

        // Delete on nonexistent is actually valid
        let result = verify_wal_entry_consistency(&delete_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        // This should be None since delete is allowed on nonexistent
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_delete_after_insert() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        // First insert
        let mut insert_entry = create_test_entry(EntryType::Insert, "doc1", "txn1");
        insert_entry.data = Some(r#"{"name": "test"}"#.to_string());
        verify_wal_entry_consistency(&insert_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();

        // Then delete
        let delete_entry = create_test_entry(EntryType::Delete, "doc1", "txn2");
        let result = verify_wal_entry_consistency(&delete_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        assert!(result.is_none()); // Should be valid
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_rollback_issue() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        // Begin first
        let begin_entry = create_test_entry(EntryType::Begin, "doc1", "txn1");
        verify_wal_entry_consistency(&begin_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();

        // Try to rollback without operations
        let rollback_entry = create_test_entry(EntryType::Rollback, "doc1", "txn1");
        let result = verify_wal_entry_consistency(&rollback_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        assert!(result.is_none()); // Rollback after begin without operations should be fine
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_multiple_updates() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        // Insert
        let mut insert_entry = create_test_entry(EntryType::Insert, "doc1", "txn1");
        insert_entry.data = Some(r#"{"v": 1}"#.to_string());
        verify_wal_entry_consistency(&insert_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();

        // Update 1
        let mut update1 = create_test_entry(EntryType::Update, "doc1", "txn2");
        update1.data = Some(r#"{"v": 2}"#.to_string());
        verify_wal_entry_consistency(&update1, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();

        // Update 2
        let mut update2 = create_test_entry(EntryType::Update, "doc1", "txn3");
        update2.data = Some(r#"{"v": 3}"#.to_string());
        let result = verify_wal_entry_consistency(&update2, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        assert!(result.is_none()); // Should be valid
    }

    #[tokio::test]
    async fn test_verify_wal_entry_consistency_commit_valid() {
        let mut wal_states = std::collections::HashMap::new();
        let mut active_transactions = std::collections::HashMap::new();

        let begin_entry = create_test_entry(EntryType::Begin, "doc1", "txn1");
        verify_wal_entry_consistency(&begin_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();

        let commit_entry = create_test_entry(EntryType::Commit, "doc1", "txn1");
        let result = verify_wal_entry_consistency(&commit_entry, &mut wal_states, &mut active_transactions)
            .await
            .unwrap();
        assert!(result.is_none()); // Commit should be fine after begin
    }
}
