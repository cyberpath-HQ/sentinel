use std::path::PathBuf;

use serde_json::Value;
use tokio::fs as tokio_fs;

use crate::{Document, Result, SentinelError};

pub struct Collection {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
}

impl Collection {
    /// Validates that a document ID is filename-safe across platforms.
    ///
    /// Document IDs must:
    /// - Not be empty
    /// - Not contain path separators (/, \)
    /// - Not contain special filesystem characters (?, *, :, |, <, >, ")
    /// - Not contain control characters
    /// - Not be "." or ".."
    /// - Only contain alphanumeric characters, hyphens, underscores, and periods
    ///
    /// # Arguments
    /// * `id` - The document ID to validate
    ///
    /// # Returns
    /// * `Ok(())` if the ID is valid
    /// * `Err(SentinelError::InvalidDocumentId)` if the ID contains unsafe characters
    #[doc(hidden)]
    pub fn validate_document_id(id: &str) -> Result<()> {
        // Check if empty
        if id.is_empty() {
            return Err(SentinelError::InvalidDocumentId {
                id: id.to_owned(),
            });
        }

        // Check for reserved names
        if id == "." || id == ".." {
            return Err(SentinelError::InvalidDocumentId {
                id: id.to_owned(),
            });
        }

        // Check for unsafe characters
        // Only allow: ASCII alphanumeric, hyphen, underscore, period
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(SentinelError::InvalidDocumentId {
                id: id.to_owned(),
            });
        }

        Ok(())
    }

    pub async fn insert(&self, id: &str, data: Value) -> Result<()> {
        Self::validate_document_id(id)?;
        let file_path = self.path.join(format!("{}.json", id));
        let json = serde_json::to_string_pretty(&data)?;
        tokio_fs::write(&file_path, json).await?;
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Result<Option<Document>> {
        Self::validate_document_id(id)?;
        let file_path = self.path.join(format!("{}.json", id));
        match tokio_fs::read_to_string(&file_path).await {
            Ok(content) => {
                let data: Value = serde_json::from_str(&content)?;
                Ok(Some(Document {
                    id: id.to_string(),
                    data,
                }))
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => {
                Err(SentinelError::Io {
                    source: e,
                })
            },
        }
    }

    pub async fn update(&self, id: &str, data: Value) -> Result<()> {
        // For update, just insert (overwrite)
        self.insert(id, data).await
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        Self::validate_document_id(id)?;
        let file_path = self.path.join(format!("{}.json", id));
        match tokio_fs::remove_file(&file_path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()), // Already deleted
            Err(e) => {
                Err(SentinelError::Io {
                    source: e,
                })
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::tempdir;

    use super::*;
    use crate::Store;

    /// Helper function to set up a temporary collection for testing
    async fn setup_collection() -> (Collection, tempfile::TempDir) {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();
        (collection, temp_dir)
    }

    #[tokio::test]
    async fn test_insert_and_retrieve() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "name": "Alice", "email": "alice@example.com" });
        collection.insert("user-123", doc.clone()).await.unwrap();

        let retrieved = collection.get("user-123").await.unwrap();
        assert_eq!(retrieved.unwrap().data, doc);
    }

    #[tokio::test]
    async fn test_insert_empty_document() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({});
        collection.insert("empty", doc.clone()).await.unwrap();

        let retrieved = collection.get("empty").await.unwrap();
        assert_eq!(retrieved.unwrap().data, doc);
    }

    #[tokio::test]
    async fn test_insert_large_document() {
        let (collection, _temp_dir) = setup_collection().await;

        let large_data = json!({
            "large_array": (0..1000).collect::<Vec<_>>(),
            "nested": {
                "deep": {
                    "value": "test"
                }
            }
        });
        collection
            .insert("large", large_data.clone())
            .await
            .unwrap();

        let retrieved = collection.get("large").await.unwrap();
        assert_eq!(retrieved.unwrap().data, large_data);
    }

    #[tokio::test]
    async fn test_insert_with_valid_special_characters_in_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "data": "test" });
        // These characters are allowed: alphanumeric, hyphen, underscore, period
        collection
            .insert("user_123-special.v1", doc.clone())
            .await
            .unwrap();

        let retrieved = collection.get("user_123-special.v1").await.unwrap();
        assert_eq!(retrieved.unwrap().data, doc);
    }

    #[tokio::test]
    async fn test_insert_with_invalid_characters_rejected() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "data": "test" });

        // Test various unsafe characters
        let unsafe_ids = vec![
            "user!123",       // exclamation mark
            "user?123",       // question mark
            "user*123",       // asterisk
            "user/123",       // forward slash
            "user\\123",      // backslash
            "user:123",       // colon
            "user|123",       // pipe
            "user<123",       // less than
            "user>123",       // greater than
            "user\"123",      // double quote
            "user 123",       // space
            "",               // empty
            ".",              // dot
            "..",             // double dot
        ];

        for unsafe_id in unsafe_ids {
            let result = collection.insert(unsafe_id, doc.clone()).await;
            assert!(
                result.is_err(),
                "Expected error for unsafe ID: '{}'",
                unsafe_id
            );
            assert!(
                matches!(result, Err(SentinelError::InvalidDocumentId { .. })),
                "Expected InvalidDocumentId error for: '{}'",
                unsafe_id
            );
        }
    }

    #[tokio::test]
    async fn test_get_with_invalid_id_rejected() {
        let (collection, _temp_dir) = setup_collection().await;

        let result = collection.get("user!123").await;
        assert!(result.is_err());
        assert!(matches!(result, Err(SentinelError::InvalidDocumentId { .. })));
    }

    #[tokio::test]
    async fn test_update_with_invalid_id_rejected() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "data": "test" });
        let result = collection.update("user!123", doc).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(SentinelError::InvalidDocumentId { .. })));
    }

    #[tokio::test]
    async fn test_delete_with_invalid_id_rejected() {
        let (collection, _temp_dir) = setup_collection().await;

        let result = collection.delete("user!123").await;
        assert!(result.is_err());
        assert!(matches!(result, Err(SentinelError::InvalidDocumentId { .. })));
    }

    /// Unit test for validate_document_id with valid IDs
    #[test]
    fn test_validate_document_id_valid() {
        // Valid IDs
        Collection::validate_document_id("user-123").unwrap();
        Collection::validate_document_id("user_123").unwrap();
        Collection::validate_document_id("user.123").unwrap();
        Collection::validate_document_id("abc").unwrap();
        Collection::validate_document_id("ABC").unwrap();
        Collection::validate_document_id("123").unwrap();
        Collection::validate_document_id("a-b_c.d").unwrap();
        Collection::validate_document_id("user-123-special.v1").unwrap();
    }

    /// Unit test for validate_document_id with invalid IDs
    #[test]
    fn test_validate_document_id_invalid() {
        // Invalid IDs
        assert!(Collection::validate_document_id("").is_err());
        assert!(Collection::validate_document_id(".").is_err());
        assert!(Collection::validate_document_id("..").is_err());
        assert!(Collection::validate_document_id("user!123").is_err());
        assert!(Collection::validate_document_id("user?123").is_err());
        assert!(Collection::validate_document_id("user*123").is_err());
        assert!(Collection::validate_document_id("user/123").is_err());
        assert!(Collection::validate_document_id("user\\123").is_err());
        assert!(Collection::validate_document_id("user:123").is_err());
        assert!(Collection::validate_document_id("user|123").is_err());
        assert!(Collection::validate_document_id("user<123").is_err());
        assert!(Collection::validate_document_id("user>123").is_err());
        assert!(Collection::validate_document_id("user\"123").is_err());
        assert!(Collection::validate_document_id("user 123").is_err());
        assert!(Collection::validate_document_id("user\t123").is_err());
        assert!(Collection::validate_document_id("user\n123").is_err());
    }

    /// Unit test for validate_document_id edge cases
    #[test]
    fn test_validate_document_id_edge_cases() {
        // Single characters
        Collection::validate_document_id("a").unwrap();
        Collection::validate_document_id("1").unwrap();
        Collection::validate_document_id("-").unwrap();
        Collection::validate_document_id("_").unwrap();

        // Long IDs
        let long_id = "a".repeat(255);
        Collection::validate_document_id(&long_id).unwrap();

        // Unicode (should be rejected as it's not alphanumeric ASCII)
        assert!(Collection::validate_document_id("user-caf\u{e9}").is_err());
        assert!(Collection::validate_document_id("\u{7528}\u{6237}123").is_err());
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let (collection, _temp_dir) = setup_collection().await;

        let retrieved = collection.get("nonexistent").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_update() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc1 = json!({ "name": "Alice" });
        collection.insert("user-123", doc1).await.unwrap();

        let doc2 = json!({ "name": "Alice", "age": 30 });
        collection.update("user-123", doc2.clone()).await.unwrap();

        let retrieved = collection.get("user-123").await.unwrap();
        assert_eq!(retrieved.unwrap().data, doc2);
    }

    #[tokio::test]
    async fn test_update_nonexistent() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "name": "Bob" });
        collection.update("new-user", doc.clone()).await.unwrap();

        let retrieved = collection.get("new-user").await.unwrap();
        assert_eq!(retrieved.unwrap().data, doc);
    }

    #[tokio::test]
    async fn test_delete() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "name": "Alice" });
        collection.insert("user-123", doc).await.unwrap();

        let retrieved = collection.get("user-123").await.unwrap();
        assert!(retrieved.is_some());

        collection.delete("user-123").await.unwrap();

        let retrieved = collection.get("user-123").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let (collection, _temp_dir) = setup_collection().await;

        // Should not error
        collection.delete("nonexistent").await.unwrap();
    }

    #[tokio::test]
    async fn test_multiple_operations() {
        let (collection, _temp_dir) = setup_collection().await;

        // Insert multiple
        collection
            .insert("user1", json!({"name": "User1"}))
            .await
            .unwrap();
        collection
            .insert("user2", json!({"name": "User2"}))
            .await
            .unwrap();

        // Get both
        let user1 = collection.get("user1").await.unwrap().unwrap();
        let user2 = collection.get("user2").await.unwrap().unwrap();
        assert_eq!(user1.data["name"], "User1");
        assert_eq!(user2.data["name"], "User2");

        // Update one
        collection
            .update("user1", json!({"name": "Updated"}))
            .await
            .unwrap();
        let updated = collection.get("user1").await.unwrap().unwrap();
        assert_eq!(updated.data["name"], "Updated");

        // Delete one
        collection.delete("user2").await.unwrap();
        assert!(collection.get("user2").await.unwrap().is_none());
        assert!(collection.get("user1").await.unwrap().is_some());
    }
}
