use std::path::PathBuf;

use serde_json::Value;
use tokio::fs as tokio_fs;

use crate::{Document, Result, SentinelError};

/// A collection represents a namespace for documents in the Sentinel database.
///
/// Collections are backed by filesystem directories, where each document is stored
/// as a JSON file. The collection provides CRUD operations (Create, Read, Update, Delete)
/// for managing documents asynchronously using tokio.
///
/// # Structure
///
/// Each collection is stored in a directory with the following structure:
/// - `{collection_name}/` - Root directory for the collection
/// - `{collection_name}/{id}.json` - Individual document files
///
/// # Example
///
/// ```rust
/// use sentinel::{Store, Collection};
/// use serde_json::json;
///
/// # async fn example() -> sentinel::Result<()> {
/// // Create a store and get a collection
/// let store = Store::new("/path/to/data").await?;
/// let collection = store.collection("users").await?;
///
/// // Insert a document
/// let user_data = json!({
///     "name": "Alice",
///     "email": "alice@example.com"
/// });
/// collection.insert("user-123", user_data).await?;
///
/// // Retrieve the document
/// let doc = collection.get("user-123").await?;
/// assert!(doc.is_some());
/// # Ok(())
/// # }
/// ```
pub struct Collection {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
}

impl Collection {
    /// Inserts a new document into the collection or overwrites an existing one.
    ///
    /// The document is serialized to pretty-printed JSON and written to a file named
    /// `{id}.json` within the collection's directory. If a document with the same ID
    /// already exists, it will be overwritten.
    ///
    /// # Arguments
    ///
    /// * `id` - A unique identifier for the document. This will be used as the filename
    ///          (with `.json` extension). Must be filesystem-safe.
    /// * `data` - The JSON data to store. Can be any valid `serde_json::Value`.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `SentinelError` if the operation fails
    /// (e.g., filesystem errors, serialization errors).
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel::{Store, Collection};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel::Result<()> {
    /// let store = Store::new("/path/to/data").await?;
    /// let collection = store.collection("users").await?;
    ///
    /// let user = json!({
    ///     "name": "Alice",
    ///     "email": "alice@example.com",
    ///     "age": 30
    /// });
    ///
    /// collection.insert("user-123", user).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn insert(&self, id: &str, data: Value) -> Result<()> {
        let file_path = self.path.join(format!("{}.json", id));
        let json = serde_json::to_string_pretty(&data)?;
        tokio_fs::write(&file_path, json).await?;
        Ok(())
    }

    /// Retrieves a document from the collection by its ID.
    ///
    /// Reads the JSON file corresponding to the given ID and deserializes it into
    /// a `Document` struct. If the document doesn't exist, returns `None`.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the document to retrieve.
    ///
    /// # Returns
    ///
    /// Returns:
    /// - `Ok(Some(Document))` if the document exists and was successfully read
    /// - `Ok(None)` if the document doesn't exist (file not found)
    /// - `Err(SentinelError)` if there was an error reading or parsing the document
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel::{Store, Collection};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel::Result<()> {
    /// let store = Store::new("/path/to/data").await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert a document first
    /// collection.insert("user-123", json!({"name": "Alice"})).await?;
    ///
    /// // Retrieve the document
    /// let doc = collection.get("user-123").await?;
    /// assert!(doc.is_some());
    /// assert_eq!(doc.unwrap().id, "user-123");
    ///
    /// // Try to get a non-existent document
    /// let missing = collection.get("user-999").await?;
    /// assert!(missing.is_none());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, id: &str) -> Result<Option<Document>> {
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
            Err(e) => Err(SentinelError::Io {
                source: e,
            }),
        }
    }

    /// Updates an existing document or creates a new one if it doesn't exist.
    ///
    /// This method is semantically equivalent to `insert` in the current implementation,
    /// as it overwrites the entire document. Future versions may implement partial updates
    /// or version tracking.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the document to update.
    /// * `data` - The new JSON data that will replace the existing document.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `SentinelError` if the operation fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel::{Store, Collection};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel::Result<()> {
    /// let store = Store::new("/path/to/data").await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert initial document
    /// collection.insert("user-123", json!({"name": "Alice", "age": 30})).await?;
    ///
    /// // Update the document with new data
    /// collection.update("user-123", json!({"name": "Alice", "age": 31})).await?;
    ///
    /// // Verify the update
    /// let doc = collection.get("user-123").await?.unwrap();
    /// assert_eq!(doc.data["age"], 31);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(&self, id: &str, data: Value) -> Result<()> {
        // For update, just insert (overwrite)
        self.insert(id, data).await
    }

    /// Deletes a document from the collection.
    ///
    /// Removes the JSON file corresponding to the given ID from the filesystem.
    /// If the document doesn't exist, the operation succeeds silently (idempotent).
    /// Future versions may implement soft deletes by moving files to a `.deleted/`
    /// subdirectory.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the document to delete.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success (including when the document doesn't exist),
    /// or a `SentinelError` if the operation fails due to filesystem errors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel::{Store, Collection};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel::Result<()> {
    /// let store = Store::new("/path/to/data").await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert a document
    /// collection.insert("user-123", json!({"name": "Alice"})).await?;
    ///
    /// // Delete the document
    /// collection.delete("user-123").await?;
    ///
    /// // Verify deletion
    /// let doc = collection.get("user-123").await?;
    /// assert!(doc.is_none());
    ///
    /// // Deleting again is safe (idempotent)
    /// collection.delete("user-123").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, id: &str) -> Result<()> {
        let file_path = self.path.join(format!("{}.json", id));
        match tokio_fs::remove_file(&file_path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()), // Already deleted
            Err(e) => Err(SentinelError::Io {
                source: e,
            }),
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
    async fn test_insert_with_special_characters_in_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "data": "test" });
        collection
            .insert("user_123-special!", doc.clone())
            .await
            .unwrap();

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
