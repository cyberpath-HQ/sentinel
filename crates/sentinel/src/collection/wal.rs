use sentinel_wal::WalDocumentOps;

use super::coll::Collection;

#[async_trait::async_trait]
impl WalDocumentOps for Collection {
    async fn get_document(&self, id: &str) -> sentinel_wal::Result<Option<serde_json::Value>> {
        self.get(id)
            .await
            .map(|opt| opt.map(|d| d.data().clone()))
            .map_err(|e| sentinel_wal::WalError::Io(std::io::Error::other(format!("{}", e))))
    }

    async fn apply_operation(
        &self,
        entry_type: &sentinel_wal::EntryType,
        id: &str,
        data: Option<serde_json::Value>,
    ) -> sentinel_wal::Result<()> {
        match *entry_type {
            sentinel_wal::EntryType::Insert => {
                if let Some(data) = data {
                    self.insert(id, data)
                        .await
                        .map_err(|e| sentinel_wal::WalError::Io(std::io::Error::other(format!("{}", e))))
                }
                else {
                    Err(sentinel_wal::WalError::InvalidEntry(
                        "Insert operation missing data".to_string(),
                    ))
                }
            },
            sentinel_wal::EntryType::Update => {
                if let Some(data) = data {
                    self.update(id, data)
                        .await
                        .map_err(|e| sentinel_wal::WalError::Io(std::io::Error::other(format!("{}", e))))
                }
                else {
                    Err(sentinel_wal::WalError::InvalidEntry(
                        "Update operation missing data".to_string(),
                    ))
                }
            },
            sentinel_wal::EntryType::Delete => {
                self.delete(id)
                    .await
                    .map_err(|e| sentinel_wal::WalError::Io(std::io::Error::other(format!("{}", e))))
            },
            _ => Ok(()), // Other operations not handled here
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
        let store = Store::new(temp_dir.path().to_path_buf(), None)
            .await
            .unwrap();
        let collection_name = "test_collection".to_string();
        let _ = store.collection(&collection_name).await.unwrap();
        (temp_dir, store, collection_name)
    }

    #[tokio::test]
    async fn test_wal_document_ops_get_document() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Test getting a non-existent document
        let result = collection.get("nonexistent").await.unwrap();
        assert!(result.is_none());

        // Test getting a document via WAL ops (indirectly through get_document)
        let doc_id = "test-doc-1".to_string();
        let doc_data = serde_json::json!({"name": "Test", "value": 42});
        collection.insert(&doc_id, doc_data.clone()).await.unwrap();

        // Now get it via the trait method
        let result = collection.get_document(&doc_id).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), doc_data);
    }

    #[tokio::test]
    async fn test_wal_document_ops_apply_operation_insert() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        let doc_id = "test-insert-doc";
        let doc_data = serde_json::json!({"operation": "insert", "value": 100});

        // Apply insert operation
        collection
            .apply_operation(
                &sentinel_wal::EntryType::Insert,
                doc_id,
                Some(doc_data.clone()),
            )
            .await
            .unwrap();

        // Verify the document was inserted
        let result = collection.get(doc_id).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().data(), &doc_data);
    }

    #[tokio::test]
    async fn test_wal_document_ops_apply_operation_insert_missing_data() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Apply insert operation without data - should fail
        let result = collection
            .apply_operation(&sentinel_wal::EntryType::Insert, "doc-id", None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, sentinel_wal::WalError::InvalidEntry(_)));
    }

    #[tokio::test]
    async fn test_wal_document_ops_apply_operation_update() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // First insert a document
        let doc_id = "test-update-doc";
        let initial_data = serde_json::json!({"status": "initial"});
        collection
            .insert(doc_id, initial_data.clone())
            .await
            .unwrap();

        // Now update it via WAL ops
        let updated_data = serde_json::json!({"status": "updated", "value": 200});
        collection
            .apply_operation(
                &sentinel_wal::EntryType::Update,
                doc_id,
                Some(updated_data.clone()),
            )
            .await
            .unwrap();

        // Verify the document was updated
        let result = collection.get(doc_id).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().data(), &updated_data);
    }

    #[tokio::test]
    async fn test_wal_document_ops_apply_operation_update_missing_data() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Apply update operation without data - should fail
        let result = collection
            .apply_operation(&sentinel_wal::EntryType::Update, "doc-id", None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, sentinel_wal::WalError::InvalidEntry(_)));
    }

    #[tokio::test]
    async fn test_wal_document_ops_apply_operation_delete() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // First insert a document
        let doc_id = "test-delete-doc";
        let doc_data = serde_json::json!("to_be_deleted");
        collection.insert(doc_id, doc_data).await.unwrap();

        // Verify it exists
        assert!(collection.get(doc_id).await.unwrap().is_some());

        // Delete it via WAL ops
        collection
            .apply_operation(&sentinel_wal::EntryType::Delete, doc_id, None)
            .await
            .unwrap();

        // Verify it was deleted
        assert!(collection.get(doc_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_wal_document_ops_apply_operation_delete_nonexistent() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Delete a non-existent document - should still succeed (idempotent)
        let result = collection
            .apply_operation(&sentinel_wal::EntryType::Delete, "nonexistent-doc", None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wal_document_ops_apply_operation_begin() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Begin operation should succeed but do nothing (idempotent)
        let result = collection
            .apply_operation(&sentinel_wal::EntryType::Begin, "ignored", None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wal_document_ops_apply_operation_commit() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Commit operation should succeed but do nothing (idempotent)
        let result = collection
            .apply_operation(&sentinel_wal::EntryType::Commit, "ignored", None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wal_document_ops_apply_operation_rollback() {
        let (_temp_dir, store, collection_name) = create_test_store_with_collection().await;
        let collection = store.collection(&collection_name).await.unwrap();

        // Rollback operation should succeed but do nothing (idempotent)
        let result = collection
            .apply_operation(&sentinel_wal::EntryType::Rollback, "ignored", None)
            .await;

        assert!(result.is_ok());
    }
}
