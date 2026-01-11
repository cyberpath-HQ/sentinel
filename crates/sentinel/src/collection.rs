use std::path::PathBuf;
use std::io;
use serde_json::Value;
use tokio::fs as tokio_fs;
use crate::{Document, Store};

pub struct Collection {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
}

impl Collection {
    pub async fn insert(&self, id: &str, data: Value) -> io::Result<()> {
        let file_path = self.path.join(format!("{}.json", id));
        let json = serde_json::to_string_pretty(&data)?;
        tokio_fs::write(&file_path, json).await
    }

    pub async fn get(&self, id: &str) -> io::Result<Option<Document>> {
        let file_path = self.path.join(format!("{}.json", id));
        match tokio_fs::read_to_string(&file_path).await {
            Ok(content) => {
                let data: Value = serde_json::from_str(&content)?;
                Ok(Some(Document {
                    id: id.to_string(),
                    data,
                }))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn update(&self, id: &str, data: Value) -> io::Result<()> {
        // For update, just insert (overwrite)
        self.insert(id, data).await
    }

    pub async fn delete(&self, id: &str) -> io::Result<()> {
        let file_path = self.path.join(format!("{}.json", id));
        match tokio_fs::remove_file(&file_path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()), // Already deleted
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

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
        collection.insert("large", large_data.clone()).await.unwrap();

        let retrieved = collection.get("large").await.unwrap();
        assert_eq!(retrieved.unwrap().data, large_data);
    }

    #[tokio::test]
    async fn test_insert_with_special_characters_in_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "data": "test" });
        collection.insert("user_123-special!", doc.clone()).await.unwrap();

        let retrieved = collection.get("user_123-special!").await.unwrap();
        assert_eq!(retrieved.unwrap().data, doc);
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
        collection.insert("user1", json!({"name": "User1"})).await.unwrap();
        collection.insert("user2", json!({"name": "User2"})).await.unwrap();

        // Get both
        let user1 = collection.get("user1").await.unwrap().unwrap();
        let user2 = collection.get("user2").await.unwrap().unwrap();
        assert_eq!(user1.data["name"], "User1");
        assert_eq!(user2.data["name"], "User2");

        // Update one
        collection.update("user1", json!({"name": "Updated"})).await.unwrap();
        let updated = collection.get("user1").await.unwrap().unwrap();
        assert_eq!(updated.data["name"], "Updated");

        // Delete one
        collection.delete("user2").await.unwrap();
        assert!(collection.get("user2").await.unwrap().is_none());
        assert!(collection.get("user1").await.unwrap().is_some());
    }
}