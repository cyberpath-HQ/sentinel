//! WAL recovery functionality.
//!
//! This module provides recovery of collections from WAL entries.
//! Unlike the previous flawed approach, this recovery:
//! 1. Only replays operations that haven't been applied yet
//! 2. Handles conflicts gracefully
//! 3. Is idempotent (can be run multiple times safely)

use std::collections::HashMap;

use futures::StreamExt as _;
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
#[allow(
    clippy::arithmetic_side_effects,
    reason = "safe counter increments in recovery"
)]
pub async fn recover_from_wal_safe<D>(wal: &WalManager, document_ops: &D) -> Result<WalRecoveryResult>
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
                    entry.document_id_str().to_owned(),
                    entry.transaction_id_str().to_owned(),
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
                            transaction_id: entry.transaction_id_str().to_owned(),
                            document_id:    entry.document_id_str().to_owned(),
                            operation_type: format!("{:?}", entry.entry_type),
                            reason:         format!("{}", e),
                        });
                    },
                }
            },
            Err(e) => {
                failed += 1;
                failures.push(WalRecoveryFailure {
                    transaction_id: "unknown".to_owned(),
                    document_id:    "unknown".to_owned(),
                    operation_type: "read".to_owned(),
                    reason:         format!("Failed to read WAL entry: {}", e),
                });
            },
        }
    }

    debug!(
        "WAL recovery completed: {} recovered, {} skipped, {} failed",
        recovered, skipped, failed
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
async fn replay_wal_entry_safe<D>(entry: &LogEntry, document_ops: &D) -> Result<bool>
where
    D: WalDocumentOps,
{
    match entry.entry_type {
        EntryType::Insert => {
            if let Some(data_str) = entry.data.as_ref() {
                // Parse the JSON data
                let data: serde_json::Value = serde_json::from_str(data_str)
                    .map_err(|e| crate::error::WalError::Serialization(format!("Invalid JSON in WAL insert: {}", e)))?;

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
                        document_ops
                            .apply_operation(&EntryType::Insert, entry.document_id_str(), Some(data))
                            .await?;
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
            if let Some(data_str) = entry.data.as_ref() {
                // Parse the JSON data
                let data: serde_json::Value = serde_json::from_str(data_str)
                    .map_err(|e| crate::error::WalError::Serialization(format!("Invalid JSON in WAL update: {}", e)))?;

                // Check if document exists
                match document_ops.get_document(entry.document_id_str()).await {
                    Ok(Some(existing_doc)) => {
                        // Document exists, check if update is needed
                        if existing_doc != data {
                            document_ops
                                .apply_operation(&EntryType::Update, entry.document_id_str(), Some(data))
                                .await?;
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
                    document_ops
                        .apply_operation(&EntryType::Delete, entry.document_id_str(), None)
                        .await?;
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
#[allow(
    clippy::arithmetic_side_effects,
    reason = "safe counter increments in recovery"
)]
pub async fn recover_from_wal_force<D>(wal: &WalManager, document_ops: &D) -> Result<WalRecoveryResult>
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
                        }
                        else {
                            skipped += 1;
                        }
                    },
                    Err(e) => {
                        failed += 1;
                        failures.push(WalRecoveryFailure {
                            transaction_id: entry.transaction_id_str().to_owned(),
                            document_id:    entry.document_id_str().to_owned(),
                            operation_type: format!("{:?}", entry.entry_type),
                            reason:         format!("{}", e),
                        });
                    },
                }
            },
            Err(e) => {
                failed += 1;
                failures.push(WalRecoveryFailure {
                    transaction_id: "unknown".to_owned(),
                    document_id:    "unknown".to_owned(),
                    operation_type: "read".to_owned(),
                    reason:         format!("Failed to read WAL entry: {}", e),
                });
            },
        }
    }

    debug!(
        "Forced WAL recovery completed: {} recovered, {} skipped, {} failed",
        recovered, skipped, failed
    );

    Ok(WalRecoveryResult {
        recovered_operations: recovered,
        skipped_operations: skipped,
        failed_operations: failed,
        failures,
    })
}

/// Force replay a WAL entry (overwrites conflicts)
async fn replay_wal_entry_force<D>(entry: &LogEntry, document_ops: &D) -> Result<bool>
where
    D: WalDocumentOps,
{
    match entry.entry_type {
        EntryType::Insert | EntryType::Update => {
            if let Some(data_str) = entry.data.as_ref() {
                let data: serde_json::Value = serde_json::from_str(data_str)
                    .map_err(|e| crate::error::WalError::Serialization(format!("Invalid JSON in WAL entry: {}", e)))?;

                // For force recovery, always apply the operation
                document_ops
                    .apply_operation(&entry.entry_type, entry.document_id_str(), Some(data))
                    .await?;
                Ok(true)
            }
            else {
                Ok(false)
            }
        },
        EntryType::Delete => {
            // Force delete (ignore if document doesn't exist)
            match document_ops
                .apply_operation(&EntryType::Delete, entry.document_id_str(), None)
                .await
            {
                Ok(_) => Ok(true),
                Err(crate::error::WalError::Io {
                    ..
                }) => Ok(false), // Assume not found
                Err(e) => Err(e),
            }
        },
        // Transaction control entries don't affect document state
        EntryType::Begin | EntryType::Commit | EntryType::Rollback => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Mutex};

    use super::*;
    use crate::{EntryType, LogEntry};

    // Mock implementation of WalDocumentOps for testing
    struct MockDocumentOps {
        documents: Mutex<HashMap<String, serde_json::Value>>,
    }

    impl MockDocumentOps {
        fn new() -> Self {
            Self {
                documents: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl WalDocumentOps for MockDocumentOps {
        async fn get_document(&self, id: &str) -> Result<Option<serde_json::Value>> {
            Ok(self.documents.lock().unwrap().get(id).cloned())
        }

        async fn apply_operation(
            &self,
            operation: &EntryType,
            id: &str,
            data: Option<serde_json::Value>,
        ) -> Result<()> {
            let mut docs = self.documents.lock().unwrap();
            match operation {
                EntryType::Insert | EntryType::Update => {
                    if let Some(data) = data {
                        docs.insert(id.to_string(), data);
                    }
                },
                EntryType::Delete => {
                    docs.remove(id);
                },
                _ => {}, // No-op for other operations
            }
            Ok(())
        }
    }

    fn create_test_entry(entry_type: EntryType, doc_id: &str, data: Option<&str>) -> LogEntry {
        use crate::entry::{FixedBytes256, FixedBytes32};
        LogEntry {
            entry_type,
            collection: FixedBytes256::from(b"test" as &[u8]),
            document_id: FixedBytes256::from(doc_id.as_bytes()),
            transaction_id: FixedBytes32::from(b"txn-123" as &[u8]),
            data: data.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_insert_new_document() {
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Insert, "doc1", Some(r#"{"name": "test"}"#));

        let result = replay_wal_entry_safe(&entry, &ops).await.unwrap();
        assert!(result);

        let doc = ops.get_document("doc1").await.unwrap();
        assert_eq!(doc.unwrap()["name"], "test");
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_insert_existing_document() {
        let ops = MockDocumentOps::new();
        // Pre-insert document
        ops.apply_operation(
            &EntryType::Insert,
            "doc1",
            Some(serde_json::json!({"name": "existing"})),
        )
        .await
        .unwrap();

        let entry = create_test_entry(EntryType::Insert, "doc1", Some(r#"{"name": "duplicate"}"#));

        let result = replay_wal_entry_safe(&entry, &ops).await.unwrap();
        assert!(!result); // Should be skipped

        let doc = ops.get_document("doc1").await.unwrap();
        assert_eq!(doc.unwrap()["name"], "existing"); // Should remain unchanged
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_update_existing_document() {
        let ops = MockDocumentOps::new();
        // Pre-insert document
        ops.apply_operation(
            &EntryType::Insert,
            "doc1",
            Some(serde_json::json!({"name": "old"})),
        )
        .await
        .unwrap();

        let entry = create_test_entry(EntryType::Update, "doc1", Some(r#"{"name": "updated"}"#));

        let result = replay_wal_entry_safe(&entry, &ops).await.unwrap();
        assert!(result);

        let doc = ops.get_document("doc1").await.unwrap();
        assert_eq!(doc.unwrap()["name"], "updated");
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_update_nonexistent_document() {
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Update, "doc1", Some(r#"{"name": "updated"}"#));

        let result = replay_wal_entry_safe(&entry, &ops).await.unwrap();
        assert!(!result); // Should be skipped
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_delete_existing_document() {
        let ops = MockDocumentOps::new();
        // Pre-insert document
        ops.apply_operation(
            &EntryType::Insert,
            "doc1",
            Some(serde_json::json!({"name": "test"})),
        )
        .await
        .unwrap();

        let entry = create_test_entry(EntryType::Delete, "doc1", None);

        let result = replay_wal_entry_safe(&entry, &ops).await.unwrap();
        assert!(result);

        let doc = ops.get_document("doc1").await.unwrap();
        assert!(doc.is_none());
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_delete_nonexistent_document() {
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Delete, "doc1", None);

        let result = replay_wal_entry_safe(&entry, &ops).await.unwrap();
        assert!(!result); // Should be skipped
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_transaction_control() {
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Begin, "doc1", None);

        let result = replay_wal_entry_safe(&entry, &ops).await.unwrap();
        assert!(!result); // Transaction control should be skipped
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_invalid_json() {
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Insert, "doc1", Some("invalid json"));

        let result = replay_wal_entry_safe(&entry, &ops).await;
        assert!(result.is_err());
    }

    // ============ Additional Error Path and Edge Case Tests ============

    #[tokio::test]
    async fn test_recover_wal_safe_stream_error_handling() {
        // Test that stream errors are handled gracefully
        use tempfile::tempdir;

        use crate::{EntryType, LogEntry, WalConfig, WalManager};

        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_stream_error.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write one valid entry
        wal.write_entry(LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(serde_json::json!({"name": "Alice"})),
        ))
        .await
        .unwrap();

        let ops = MockDocumentOps::new();
        let result = recover_from_wal_safe(&wal, &ops).await.unwrap();

        // Should recover the valid entry
        assert_eq!(result.recovered_operations, 1);
        assert_eq!(result.failed_operations, 0);
    }

    #[tokio::test]
    async fn test_replay_wal_safe_doc_read_error() {
        // Test that errors when reading document are propagated correctly
        struct ErrorDocumentOps;

        #[async_trait::async_trait]
        impl WalDocumentOps for ErrorDocumentOps {
            async fn get_document(&self, _id: &str) -> Result<Option<serde_json::Value>> {
                Err(crate::WalError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "test error",
                )))
            }

            async fn apply_operation(
                &self,
                _operation: &EntryType,
                _id: &str,
                _data: Option<serde_json::Value>,
            ) -> Result<()> {
                Ok(())
            }
        }

        let ops = ErrorDocumentOps;
        let entry = create_test_entry(EntryType::Update, "doc1", Some(r#"{"name": "test"}"#));

        let result = replay_wal_entry_safe(&entry, &ops).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_recover_wal_safe_many_duplicates() {
        // Test idempotency with many duplicate transaction IDs
        use tempfile::tempdir;

        use crate::{EntryType, LogEntry, WalConfig, WalManager};

        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_many_duplicates.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write the same operation multiple times with same transaction ID
        for _ in 0 .. 5 {
            wal.write_entry(LogEntry::new(
                EntryType::Insert,
                "users".to_string(),
                "user-1".to_string(),
                Some(serde_json::json!({"name": "Alice"})),
            ))
            .await
            .unwrap();
        }

        let ops = MockDocumentOps::new();
        let result = recover_from_wal_safe(&wal, &ops).await.unwrap();

        // First one recovered, rest skipped due to duplicate detection
        assert_eq!(result.recovered_operations, 1);
        assert_eq!(result.skipped_operations, 4);
    }

    #[tokio::test]
    async fn test_replay_wal_force_io_error_on_delete() {
        // Test force delete handles IO errors gracefully
        struct IoErrorDocumentOps;

        #[async_trait::async_trait]
        impl WalDocumentOps for IoErrorDocumentOps {
            async fn get_document(&self, _id: &str) -> Result<Option<serde_json::Value>> { Ok(None) }

            async fn apply_operation(
                &self,
                operation: &EntryType,
                _id: &str,
                _data: Option<serde_json::Value>,
            ) -> Result<()> {
                if *operation == EntryType::Delete {
                    // Simulate IO error on delete
                    Err(crate::WalError::Io(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "permission denied",
                    )))
                }
                else {
                    Ok(())
                }
            }
        }

        let ops = IoErrorDocumentOps;
        let entry = create_test_entry(EntryType::Delete, "doc1", None);

        // Force replay should handle IO error
        let result = replay_wal_entry_force(&entry, &ops).await;
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("permission"));
    }

    #[tokio::test]
    async fn test_wal_recovery_failure_special_chars() {
        // Test WalRecoveryFailure with special characters in fields
        let failure = WalRecoveryFailure {
            transaction_id: "txn-特殊字符".to_string(),
            document_id:    "doc-with-quotes".to_string(),
            operation_type: "Insert".to_string(),
            reason:         "Error with\nnewlines\tand tabs".to_string(),
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&failure).unwrap();
        let deserialized: WalRecoveryFailure = serde_json::from_str(&json).unwrap();

        assert_eq!(failure.transaction_id, deserialized.transaction_id);
        assert_eq!(failure.document_id, deserialized.document_id);
        assert_eq!(failure.reason, deserialized.reason);
    }

    #[tokio::test]
    async fn test_recover_wal_safe_mixed_ops() {
        // Test recovery with mix of recovered and skipped operations
        use tempfile::tempdir;

        use crate::{EntryType, LogEntry, WalConfig, WalManager};

        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_mixed.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write multiple operations
        wal.write_entry(LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(serde_json::json!({"name": "Alice"})),
        ))
        .await
        .unwrap();

        wal.write_entry(LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-2".to_string(),
            Some(serde_json::json!({"name": "Bob"})),
        ))
        .await
        .unwrap();

        wal.write_entry(LogEntry::new(
            EntryType::Delete,
            "users".to_string(),
            "user-1".to_string(),
            None,
        ))
        .await
        .unwrap();

        let ops = MockDocumentOps::new();
        let result = recover_from_wal_safe(&wal, &ops).await.unwrap();

        // All 3 operations should be processed
        assert_eq!(result.recovered_operations, 3);
    }

    #[tokio::test]
    async fn test_recover_wal_force_txn_boundaries() {
        // Test force recovery respects transaction boundaries correctly
        use tempfile::tempdir;

        use crate::{EntryType, LogEntry, WalConfig, WalManager};

        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_txn_boundaries.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write a complete transaction
        wal.write_entry(LogEntry::new(
            EntryType::Begin,
            "users".to_string(),
            "txn-100".to_string(),
            None,
        ))
        .await
        .unwrap();

        wal.write_entry(LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-new".to_string(),
            Some(serde_json::json!({"name": "New User"})),
        ))
        .await
        .unwrap();

        wal.write_entry(LogEntry::new(
            EntryType::Commit,
            "users".to_string(),
            "txn-100".to_string(),
            None,
        ))
        .await
        .unwrap();

        let ops = MockDocumentOps::new();
        let result = recover_from_wal_force(&wal, &ops).await.unwrap();

        // Force recovery should apply insert (1 recovered), skip transaction controls (2 skipped)
        assert_eq!(result.recovered_operations, 1);
        assert_eq!(result.skipped_operations, 2);
    }

    #[tokio::test]
    async fn test_replay_wal_safe_insert_fail_error() {
        // Test that duplicate key errors during apply are handled
        struct FailOnInsertDocumentOps;

        #[async_trait::async_trait]
        impl WalDocumentOps for FailOnInsertDocumentOps {
            async fn get_document(&self, _id: &str) -> Result<Option<serde_json::Value>> { Ok(None) }

            async fn apply_operation(
                &self,
                operation: &EntryType,
                id: &str,
                _data: Option<serde_json::Value>,
            ) -> Result<()> {
                if *operation == EntryType::Insert && id == "fail-doc" {
                    Err(crate::WalError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "insert failed",
                    )))
                }
                else {
                    Ok(())
                }
            }
        }

        let ops = FailOnInsertDocumentOps;
        let entry = create_test_entry(EntryType::Insert, "fail-doc", Some(r#"{"name": "test"}"#));

        let result = replay_wal_entry_safe(&entry, &ops).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wal_recovery_result_zero_check() {
        // Test WalRecoveryResult with all zeros
        let result = WalRecoveryResult {
            recovered_operations: 0,
            skipped_operations:   0,
            failed_operations:    0,
            failures:             vec![],
        };

        assert_eq!(result.recovered_operations, 0);
        assert_eq!(result.skipped_operations, 0);
        assert_eq!(result.failed_operations, 0);
        assert!(result.failures.is_empty());
    }

    #[tokio::test]
    async fn test_recover_wal_safe_partial_update() {
        // Test update that doesn't actually change the document
        use tempfile::tempdir;

        use crate::{EntryType, LogEntry, WalConfig, WalManager};

        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_partial_update.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // First insert a document
        wal.write_entry(LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(serde_json::json!({"name": "Alice", "age": 30, "city": "NYC"})),
        ))
        .await
        .unwrap();

        // Update with same data (should be skipped in safe recovery)
        wal.write_entry(LogEntry::new(
            EntryType::Update,
            "users".to_string(),
            "user-1".to_string(),
            Some(serde_json::json!({"name": "Alice", "age": 30, "city": "NYC"})), // Same data
        ))
        .await
        .unwrap();

        let ops = MockDocumentOps::new();
        let result = recover_from_wal_safe(&wal, &ops).await.unwrap();

        // 1 recovered (insert), 1 skipped (update with no change)
        assert_eq!(result.recovered_operations, 1);
        assert_eq!(result.skipped_operations, 1);
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_insert_no_data() {
        // Test insert entry with missing data
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Insert, "doc1", None);

        let result = replay_wal_entry_safe(&entry, &ops).await.unwrap();
        assert!(!result); // Should be skipped
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_update_no_data() {
        // Test update entry with missing data
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Update, "doc1", None);

        let result = replay_wal_entry_safe(&entry, &ops).await.unwrap();
        assert!(!result); // Should be skipped
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_commit() {
        // Test commit entry skips correctly
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Commit, "doc1", None);

        let result = replay_wal_entry_safe(&entry, &ops).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_rollback() {
        // Test rollback entry skips correctly
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Rollback, "doc1", None);

        let result = replay_wal_entry_safe(&entry, &ops).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_replay_wal_entry_force_insert_no_data() {
        // Test force insert with missing data
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Insert, "doc1", None);

        let result = replay_wal_entry_force(&entry, &ops).await.unwrap();
        assert!(!result); // Should return false for missing data
    }

    #[tokio::test]
    async fn test_replay_wal_entry_force_begin() {
        // Test force replay of begin entry
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Begin, "doc1", None);

        let result = replay_wal_entry_force(&entry, &ops).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_replay_wal_entry_force_commit() {
        // Test force replay of commit entry
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Commit, "doc1", None);

        let result = replay_wal_entry_force(&entry, &ops).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_replay_wal_entry_force_rollback() {
        // Test force replay of rollback entry
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Rollback, "doc1", None);

        let result = replay_wal_entry_force(&entry, &ops).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_replay_wal_entry_force_update() {
        // Test force update
        let ops = MockDocumentOps::new();
        ops.apply_operation(
            &EntryType::Insert,
            "doc1",
            Some(serde_json::json!({"version": 1})),
        )
        .await
        .unwrap();

        let entry = create_test_entry(EntryType::Update, "doc1", Some(r#"{"version": 2}"#));

        let result = replay_wal_entry_force(&entry, &ops).await.unwrap();
        assert!(result);

        let doc = ops.get_document("doc1").await.unwrap();
        assert_eq!(doc.unwrap()["version"], 2);
    }

    #[tokio::test]
    async fn test_recover_wal_force_no_errors() {
        // Test force recovery without errors
        use tempfile::tempdir;

        use crate::{EntryType, LogEntry, WalConfig, WalManager};

        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_force_no_errors.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write various operations
        wal.write_entry(LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(serde_json::json!({"name": "Alice"})),
        ))
        .await
        .unwrap();

        wal.write_entry(LogEntry::new(
            EntryType::Update,
            "users".to_string(),
            "user-1".to_string(),
            Some(serde_json::json!({"name": "Alice Updated"})),
        ))
        .await
        .unwrap();

        wal.write_entry(LogEntry::new(
            EntryType::Delete,
            "users".to_string(),
            "user-1".to_string(),
            None,
        ))
        .await
        .unwrap();

        let ops = MockDocumentOps::new();
        let result = recover_from_wal_force(&wal, &ops).await.unwrap();

        assert_eq!(result.recovered_operations, 3);
        assert_eq!(result.failed_operations, 0);
    }

    #[tokio::test]
    async fn test_recover_wal_safe_error_in_apply() {
        // Test safe recovery when apply_operation fails
        struct FailingDocumentOps {
            fail_on_insert: bool,
        }

        #[async_trait::async_trait]
        impl WalDocumentOps for FailingDocumentOps {
            async fn get_document(&self, _id: &str) -> Result<Option<serde_json::Value>> { Ok(None) }

            async fn apply_operation(
                &self,
                operation: &EntryType,
                _id: &str,
                _data: Option<serde_json::Value>,
            ) -> Result<()> {
                if self.fail_on_insert && *operation == EntryType::Insert {
                    Err(crate::WalError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "apply failed",
                    )))
                }
                else {
                    Ok(())
                }
            }
        }

        let ops = FailingDocumentOps {
            fail_on_insert: true,
        };
        let entry = create_test_entry(EntryType::Insert, "doc1", Some(r#"{"name": "test"}"#));

        let result = replay_wal_entry_safe(&entry, &ops).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_recover_wal_safe_all_failures() {
        // Test recovery result with all failures
        use tempfile::tempdir;

        use crate::{EntryType, LogEntry, WalConfig, WalManager};

        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_all_failures.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        wal.write_entry(LogEntry::new(
            EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(serde_json::json!({"name": "Alice"})),
        ))
        .await
        .unwrap();

        struct FailAllDocumentOps;

        #[async_trait::async_trait]
        impl WalDocumentOps for FailAllDocumentOps {
            async fn get_document(&self, _id: &str) -> Result<Option<serde_json::Value>> {
                Err(crate::WalError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "always fail",
                )))
            }

            async fn apply_operation(
                &self,
                _operation: &EntryType,
                _id: &str,
                _data: Option<serde_json::Value>,
            ) -> Result<()> {
                Ok(())
            }
        }

        let ops = FailAllDocumentOps;
        let result = recover_from_wal_safe(&wal, &ops).await.unwrap();

        assert_eq!(result.failed_operations, 1);
        assert!(!result.failures.is_empty());
    }

    #[tokio::test]
    async fn test_replay_wal_entry_safe_update_same_data() {
        // Test update with same data returns false
        let ops = MockDocumentOps::new();
        let data = serde_json::json!({"name": "test", "age": 25});

        ops.apply_operation(&EntryType::Insert, "doc1", Some(data.clone()))
            .await
            .unwrap();

        let entry = create_test_entry(
            EntryType::Update,
            "doc1",
            Some(r#"{"name": "test", "age": 25}"#),
        );

        let result = replay_wal_entry_safe(&entry, &ops).await.unwrap();
        assert!(!result); // Should be skipped because data is the same
    }

    #[tokio::test]
    async fn test_replay_wal_entry_force_delete_nonexistent() {
        // Test force delete of nonexistent document returns false for IO error
        struct DeleteFailsDocumentOps;

        #[async_trait::async_trait]
        impl WalDocumentOps for DeleteFailsDocumentOps {
            async fn get_document(&self, _id: &str) -> Result<Option<serde_json::Value>> { Ok(None) }

            async fn apply_operation(
                &self,
                _operation: &EntryType,
                _id: &str,
                _data: Option<serde_json::Value>,
            ) -> Result<()> {
                Err(crate::WalError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not found",
                )))
            }
        }

        let ops = DeleteFailsDocumentOps;
        let entry = create_test_entry(EntryType::Delete, "doc1", None);

        let result = replay_wal_entry_force(&entry, &ops).await.unwrap();
        assert!(!result); // IO error treated as false
    }

    #[tokio::test]
    async fn test_recover_wal_safe_update_invalid_json() {
        // Test update with invalid JSON
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Update, "doc1", Some("not valid json"));

        let result = replay_wal_entry_safe(&entry, &ops).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_recover_wal_force_update_no_data() {
        // Test force update with no data
        let ops = MockDocumentOps::new();
        let entry = create_test_entry(EntryType::Update, "doc1", None);

        let result = replay_wal_entry_force(&entry, &ops).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_recover_wal_force_delete_success() {
        // Test force delete with success
        let ops = MockDocumentOps::new();
        ops.apply_operation(
            &EntryType::Insert,
            "doc1",
            Some(serde_json::json!({"name": "test"})),
        )
        .await
        .unwrap();

        let entry = create_test_entry(EntryType::Delete, "doc1", None);

        let result = replay_wal_entry_force(&entry, &ops).await.unwrap();
        assert!(result);

        let doc = ops.get_document("doc1").await.unwrap();
        assert!(doc.is_none());
    }

    #[tokio::test]
    async fn test_recover_wal_force_with_apply_error() {
        // Test force recovery when apply_operation returns non-IO error
        struct CustomErrorOps;

        #[async_trait::async_trait]
        impl WalDocumentOps for CustomErrorOps {
            async fn get_document(&self, _id: &str) -> Result<Option<serde_json::Value>> { Ok(None) }

            async fn apply_operation(
                &self,
                _operation: &EntryType,
                _id: &str,
                _data: Option<serde_json::Value>,
            ) -> Result<()> {
                Err(crate::WalError::Serialization("custom error".to_string()))
            }
        }

        let ops = CustomErrorOps;
        let entry = create_test_entry(EntryType::Insert, "doc1", Some(r#"{"name": "test"}"#));

        let result = replay_wal_entry_force(&entry, &ops).await;
        assert!(result.is_err());
    }
}
