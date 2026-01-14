use std::path::PathBuf;

use serde_json::Value;
use tokio::fs as tokio_fs;

use crate::{
    validation::{is_reserved_name, is_valid_document_id_chars},
    Document,
    Result,
    SentinelError,
};

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
#[derive(Debug)]
#[allow(
    clippy::field_scoped_visibility_modifiers,
    reason = "fields need to be pub(crate) for internal access"
)]
pub struct Collection {
    /// The filesystem path to the collection directory.
    pub(crate) path: PathBuf,
}

impl Collection {
    /// Returns the name of the collection.
    pub fn name(&self) -> &str { self.path.file_name().unwrap().to_str().unwrap() }

    /// Inserts a new document into the collection or overwrites an existing one.
    ///
    /// The document is serialized to pretty-printed JSON and written to a file named
    /// `{id}.json` within the collection's directory. If a document with the same ID
    /// already exists, it will be overwritten.
    ///
    /// # Arguments
    ///
    /// * `id` - A unique identifier for the document. This will be used as the filename (with
    ///   `.json` extension). Must be filesystem-safe.
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
        validate_document_id(id)?;
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
    /// assert_eq!(doc.unwrap().id(), "user-123");
    ///
    /// // Try to get a non-existent document
    /// let missing = collection.get("user-999").await?;
    /// assert!(missing.is_none());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, id: &str) -> Result<Option<Document>> {
        validate_document_id(id)?;
        let file_path = self.path.join(format!("{}.json", id));
        match tokio_fs::read_to_string(&file_path).await {
            Ok(content) => {
                let data: Value = serde_json::from_str(&content)?;
                Ok(Some(Document {
                    id: id.to_owned(),
                    data,
                    ..Default::default()
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
    /// assert_eq!(doc.data()["age"], 31);
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
        validate_document_id(id)?;
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

/// Validates that a document ID is filesystem-safe across all platforms.
///
/// # Rules
/// - Must not be empty
/// - Must not contain path separators (`/` or `\`)
/// - Must not contain control characters (0x00-0x1F, 0x7F)
/// - Must not be a Windows reserved name (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
/// - Must not contain Windows reserved characters (< > : " | ? *)
/// - Must only contain valid filename characters
///
/// # Parameters
/// - `id`: The document ID to validate
///
/// # Returns
/// - `Ok(())` if the ID is valid
/// - `Err(SentinelError::InvalidDocumentId)` if the ID is invalid
pub(crate) fn validate_document_id(id: &str) -> Result<()> {
    // Check if id is empty
    if id.is_empty() {
        return Err(SentinelError::InvalidDocumentId {
            id: id.to_owned(),
        });
    }

    // Check for valid characters
    if !is_valid_document_id_chars(id) {
        return Err(SentinelError::InvalidDocumentId {
            id: id.to_owned(),
        });
    }

    // Check for Windows reserved names
    if is_reserved_name(id) {
        return Err(SentinelError::InvalidDocumentId {
            id: id.to_owned(),
        });
    }

    Ok(())
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
        assert_eq!(*retrieved.unwrap().data(), doc);
    }

    #[tokio::test]
    async fn test_insert_empty_document() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({});
        collection.insert("empty", doc.clone()).await.unwrap();

        let retrieved = collection.get("empty").await.unwrap();
        assert_eq!(*retrieved.unwrap().data(), doc);
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
        assert_eq!(*retrieved.unwrap().data(), large_data);
    }

    #[tokio::test]
    async fn test_insert_with_invalid_special_characters_in_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "data": "test" });
        let result = collection.insert("user_123-special!", doc.clone()).await;

        // Should return an error for invalid document ID with special characters
        assert!(result.is_err());
        match result {
            Err(SentinelError::InvalidDocumentId {
                id,
            }) => {
                assert_eq!(id, "user_123-special!");
            },
            _ => panic!("Expected InvalidDocumentId error"),
        }
    }

    #[tokio::test]
    async fn test_insert_with_valid_document_ids() {
        let (collection, _temp_dir) = setup_collection().await;

        // Test various valid document IDs
        let valid_ids = vec![
            "user-123",
            "user_456",
            "user123",
            "123",
            "a",
            "user-123_test",
            "user_123-test",
            "CamelCaseID",
            "lower_case_id",
            "UPPER_CASE_ID",
        ];

        for id in valid_ids {
            let doc = json!({ "data": "test" });
            let result = collection.insert(id, doc).await;
            assert!(
                result.is_ok(),
                "Expected ID '{}' to be valid but got error: {:?}",
                id,
                result
            );
        }
    }

    #[tokio::test]
    async fn test_insert_with_various_invalid_document_ids() {
        let (collection, _temp_dir) = setup_collection().await;

        // Test various invalid document IDs
        let invalid_ids = vec![
            "user!123",    // exclamation mark
            "user@domain", // at sign
            "user#123",    // hash
            "user$123",    // dollar sign
            "user%123",    // percent
            "user^123",    // caret
            "user&123",    // ampersand
            "user*123",    // asterisk
            "user(123)",   // parentheses
            "user.123",    // period
            "user/123",    // forward slash
            "user\\123",   // backslash
            "user:123",    // colon
            "user;123",    // semicolon
            "user<123",    // less than
            "user>123",    // greater than
            "user?123",    // question mark
            "user|123",    // pipe
            "user\"123",   // quote
            "user'123",    // single quote
            "",            // empty string
        ];

        for id in invalid_ids {
            let doc = json!({ "data": "test" });
            let result = collection.insert(id, doc).await;
            assert!(
                result.is_err(),
                "Expected ID '{}' to be invalid but insertion succeeded",
                id
            );
            match result {
                Err(SentinelError::InvalidDocumentId {
                    ..
                }) => {
                    // Expected error type
                },
                _ => panic!("Expected InvalidDocumentId error for ID '{}'", id),
            }
        }
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
        assert_eq!(*retrieved.unwrap().data(), doc2);
    }

    #[tokio::test]
    async fn test_update_nonexistent() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "name": "Bob" });
        collection.update("new-user", doc.clone()).await.unwrap();

        let retrieved = collection.get("new-user").await.unwrap();
        assert_eq!(*retrieved.unwrap().data(), doc);
    }

    #[tokio::test]
    async fn test_update_with_invalid_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "name": "Bob" });
        let result = collection.update("user!invalid", doc).await;

        // Should return an error for invalid document ID
        assert!(result.is_err());
        match result {
            Err(SentinelError::InvalidDocumentId {
                id,
            }) => {
                assert_eq!(id, "user!invalid");
            },
            _ => panic!("Expected InvalidDocumentId error"),
        }
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
        assert_eq!(user1.data()["name"], "User1");
        assert_eq!(user2.data()["name"], "User2");

        // Update one
        collection
            .update("user1", json!({"name": "Updated"}))
            .await
            .unwrap();
        let updated = collection.get("user1").await.unwrap().unwrap();
        assert_eq!(updated.data()["name"], "Updated");

        // Delete one
        collection.delete("user2").await.unwrap();
        assert!(collection.get("user2").await.unwrap().is_none());
        assert!(collection.get("user1").await.unwrap().is_some());
    }

    #[test]
    fn test_validate_document_id_valid() {
        // Valid IDs
        assert!(validate_document_id("user-123").is_ok());
        assert!(validate_document_id("user_456").is_ok());
        assert!(validate_document_id("data-item").is_ok());
        assert!(validate_document_id("test_collection_123").is_ok());
        assert!(validate_document_id("file-txt").is_ok());
        assert!(validate_document_id("a").is_ok());
        assert!(validate_document_id("123").is_ok());
    }

    #[test]
    fn test_validate_document_id_invalid_empty() {
        assert!(validate_document_id("").is_err());
    }

    #[test]
    fn test_validate_document_id_invalid_path_separators() {
        assert!(validate_document_id("path/traversal").is_err());
        assert!(validate_document_id("path\\traversal").is_err());
    }

    #[test]
    fn test_validate_document_id_invalid_control_characters() {
        assert!(validate_document_id("file\nname").is_err());
        assert!(validate_document_id("file\x00name").is_err());
    }

    #[test]
    fn test_validate_document_id_invalid_windows_reserved_characters() {
        assert!(validate_document_id("file<name>").is_err());
        assert!(validate_document_id("file>name").is_err());
        assert!(validate_document_id("file:name").is_err());
        assert!(validate_document_id("file\"name").is_err());
        assert!(validate_document_id("file|name").is_err());
        assert!(validate_document_id("file?name").is_err());
        assert!(validate_document_id("file*name").is_err());
    }

    #[test]
    fn test_validate_document_id_invalid_other_characters() {
        assert!(validate_document_id("file name").is_err()); // space
        assert!(validate_document_id("file@name").is_err()); // @
        assert!(validate_document_id("file!name").is_err()); // !
    }

    #[test]
    fn test_validate_document_id_invalid_windows_reserved_names() {
        // Test reserved names (case-insensitive)
        assert!(validate_document_id("CON").is_err());
        assert!(validate_document_id("con").is_err());
        assert!(validate_document_id("Con").is_err());
        assert!(validate_document_id("PRN").is_err());
        assert!(validate_document_id("AUX").is_err());
        assert!(validate_document_id("NUL").is_err());
        assert!(validate_document_id("COM1").is_err());
        assert!(validate_document_id("LPT9").is_err());

        // Test with extensions
        assert!(validate_document_id("CON.txt").is_err());
        assert!(validate_document_id("prn.backup").is_err());
    }

    #[tokio::test]
    async fn test_insert_invalid_document_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "data": "test" });

        // Test empty ID
        assert!(collection.insert("", doc.clone()).await.is_err());

        // Test Windows reserved name
        assert!(collection.insert("CON", doc.clone()).await.is_err());

        // Test invalid character
        assert!(collection.insert("file name", doc.clone()).await.is_err());
    }

    #[tokio::test]
    async fn test_get_invalid_document_id() {
        let (collection, _temp_dir) = setup_collection().await;

        // Test empty ID
        assert!(collection.get("").await.is_err());

        // Test Windows reserved name
        assert!(collection.get("CON").await.is_err());
    }

    #[tokio::test]
    async fn test_update_invalid_document_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "data": "test" });

        // Test empty ID
        assert!(collection.update("", doc.clone()).await.is_err());

        // Test Windows reserved name
        assert!(collection.update("CON", doc.clone()).await.is_err());
    }

    #[tokio::test]
    async fn test_delete_invalid_document_id() {
        let (collection, _temp_dir) = setup_collection().await;

        // Test empty ID
        assert!(collection.delete("").await.is_err());

        // Test Windows reserved name
        assert!(collection.delete("CON").await.is_err());
    }
}
