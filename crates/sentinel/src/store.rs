use std::path::{Path, PathBuf};

use tokio::fs as tokio_fs;

use crate::{Collection, Result};

/// The top-level manager for document collections in Cyberpath Sentinel.
///
/// `Store` manages the root directory where all collections are stored. It handles
/// directory creation, collection access, and serves as the entry point for all
/// document storage operations. Each `Store` instance corresponds to a single
/// filesystem-backed database.
///
/// # Architecture
///
/// The Store creates a hierarchical structure:
/// - Root directory (specified at creation)
///   - `data/` subdirectory (contains all collections)
///     - Collection directories (e.g., `users/`, `audit_logs/`)
///
/// # Examples
///
/// ```no_run
/// use sentinel::Store;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create a new store at the specified path
/// let store = Store::new("/var/lib/sentinel/db").await?;
///
/// // Access a collection
/// let users = store.collection("users").await?;
/// # Ok(())
/// # }
/// ```
///
/// # Thread Safety
///
/// `Store` is safe to share across threads. Multiple collections can be accessed
/// concurrently, with each collection managing its own locking internally.
pub struct Store {
    root_path: PathBuf,
}

impl Store {
    /// Creates a new `Store` instance at the specified root path.
    ///
    /// This method initializes the store by creating the root directory if it doesn't
    /// exist. It does not create the `data/` subdirectory until collections are accessed.
    ///
    /// # Parameters
    ///
    /// * `root_path` - The filesystem path where the store will be created. This can be
    ///   any type that implements `AsRef<Path>`, including `&str`, `String`, `Path`, and
    ///   `PathBuf`.
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - Returns a new `Store` instance on success, or a `SentinelError`
    ///   if:
    ///   - The directory cannot be created due to permission issues
    ///   - The path is invalid or cannot be accessed
    ///   - I/O errors occur during directory creation
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sentinel::Store;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create a store with a string path
    /// let store = Store::new("/var/lib/sentinel").await?;
    ///
    /// // Create a store with a PathBuf
    /// use std::path::PathBuf;
    /// let path = PathBuf::from("/tmp/my-store");
    /// let store = Store::new(path).await?;
    ///
    /// // Create a store in a temporary directory
    /// let temp_dir = std::env::temp_dir().join("sentinel-test");
    /// let store = Store::new(&temp_dir).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Notes
    ///
    /// - If the directory already exists, this method succeeds without modification
    /// - Parent directories are created automatically if they don't exist
    /// - The created directory will have default permissions set by the operating system
    pub async fn new(root_path: impl AsRef<Path>) -> Result<Self> {
        let root_path = root_path.as_ref().to_path_buf();
        tokio_fs::create_dir_all(&root_path).await?;
        Ok(Self {
            root_path,
        })
    }

    /// Retrieves or creates a collection with the specified name.
    ///
    /// This method provides access to a named collection within the store. If the
    /// collection directory doesn't exist, it will be created automatically under
    /// the `data/` subdirectory of the store's root path.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the collection. This will be used as the directory name
    ///   under `data/`. The name should be filesystem-safe (avoid special characters
    ///   that are invalid in directory names on your target platform).
    ///
    /// # Returns
    ///
    /// * `Result<Collection>` - Returns a `Collection` instance on success, or a
    ///   `SentinelError` if:
    ///   - The collection directory cannot be created due to permission issues
    ///   - The name contains invalid characters for the filesystem
    ///   - I/O errors occur during directory creation
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sentinel::Store;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = Store::new("/var/lib/sentinel").await?;
    ///
    /// // Access a users collection
    /// let users = store.collection("users").await?;
    ///
    /// // Insert a document into the collection
    /// users.insert("user-123", json!({
    ///     "name": "Alice",
    ///     "email": "alice@example.com"
    /// })).await?;
    ///
    /// // Access multiple collections
    /// let audit_logs = store.collection("audit_logs").await?;
    /// let certificates = store.collection("certificates").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Collection Naming
    ///
    /// Collection names should follow these guidelines:
    /// - Use lowercase letters, numbers, underscores, and hyphens
    /// - Avoid spaces and special characters
    /// - Keep names descriptive but concise (e.g., `users`, `audit_logs`, `api_keys`)
    ///
    /// # Notes
    ///
    /// - Calling this method multiple times with the same name returns separate
    ///   `Collection` instances pointing to the same directory
    /// - The `data/` subdirectory is created automatically on first collection access
    /// - Collections are not cached; each call creates a new `Collection` instance
    /// - No validation is performed on the collection name beyond filesystem constraints
    pub async fn collection(&self, name: &str) -> Result<Collection> {
        let path = self.root_path.join("data").join(name);
        tokio_fs::create_dir_all(&path).await?;
        Ok(Collection {
            name: name.to_string(),
            path,
        })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn test_store_new_creates_directory() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");

        let _store = Store::new(&store_path).await.unwrap();
        assert!(store_path.exists());
        assert!(store_path.is_dir());
    }

    #[tokio::test]
    async fn test_store_new_with_existing_directory() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path();

        // Directory already exists
        let _store = Store::new(&store_path).await.unwrap();
        assert!(store_path.exists());
    }

    #[tokio::test]
    async fn test_store_collection_creates_subdirectory() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let collection = store.collection("users").await.unwrap();
        assert!(collection.path.exists());
        assert!(collection.path.is_dir());
        assert_eq!(collection.name, "users");
    }

    #[tokio::test]
    async fn test_store_collection_with_special_characters() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let collection = store.collection("user_data-123").await.unwrap();
        assert!(collection.path.exists());
        assert_eq!(collection.name, "user_data-123");
    }

    #[tokio::test]
    async fn test_store_collection_multiple_calls() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let coll1 = store.collection("users").await.unwrap();
        let coll2 = store.collection("users").await.unwrap();

        assert_eq!(coll1.name, coll2.name);
        assert_eq!(coll1.path, coll2.path);
    }
}
