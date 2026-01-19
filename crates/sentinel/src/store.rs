use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use serde_json::Value;
use tokio::fs as tokio_fs;
use tracing::{debug, error, trace, warn};
use sentinel_wal::WalManager;

use crate::{
    validation::{is_reserved_name, is_valid_name_chars},
    Collection,
    CollectionMetadata,
    Result,
    SentinelError,
    StoreMetadata,
    COLLECTION_METADATA_FILE,
    DATA_DIR,
    KEYS_COLLECTION,
    META_SENTINEL_VERSION,
    STORE_METADATA_FILE,
    WAL_DIR,
    WAL_FILE,
};

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
/// use sentinel_dbms::Store;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create a new store at the specified path
/// let store =
///     Store::new("/var/lib/sentinel/db", Some("my_passphrase")).await?;
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
#[derive(Debug)]
pub struct Store {
    /// The root path of the store.
    root_path:        PathBuf,
    /// The signing key for the store.
    signing_key:      Option<Arc<sentinel_crypto::SigningKey>>,
    /// When the store was created.
    created_at:       chrono::DateTime<chrono::Utc>,
    /// When the store was last accessed.
    last_accessed_at: std::sync::RwLock<chrono::DateTime<chrono::Utc>>,
    /// Total size of all collections in bytes.
    total_size_bytes: std::sync::atomic::AtomicU64,
    /// Total number of documents across all collections.
    total_documents:  std::sync::atomic::AtomicU64,
    /// Total number of collections.
    collection_count: std::sync::atomic::AtomicU64,
}

impl Store {
    /// Creates a new `Store` instance at the specified root path.
    ///
    /// This method initializes the store by creating the root directory if it doesn't
    /// exist. It does not create the `data/` subdirectory until collections are accessed.
    ///
    /// # Parameters
    ///
    /// * `root_path` - The filesystem path where the store will be created. This can be any type
    ///   that implements `AsRef<Path>`, including `&str`, `String`, `Path`, and `PathBuf`.
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - Returns a new `Store` instance on success, or a `SentinelError` if:
    ///   - The directory cannot be created due to permission issues
    ///   - The path is invalid or cannot be accessed
    ///   - I/O errors occur during directory creation
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sentinel_dbms::Store;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create a store with a string path
    /// let store = Store::new("/var/lib/sentinel", None).await?;
    ///
    /// // Create a store with a PathBuf
    /// use std::path::PathBuf;
    /// let path = PathBuf::from("/tmp/my-store");
    /// let store = Store::new(path, None).await?;
    ///
    /// // Create a store in a temporary directory
    /// let temp_dir = std::env::temp_dir().join("sentinel-test");
    /// let store = Store::new(&temp_dir, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Notes
    ///
    /// - If the directory already exists, this method succeeds without modification
    /// - Parent directories are created automatically if they don't exist
    /// - The created directory will have default permissions set by the operating system
    pub async fn new<P>(root_path: P, passphrase: Option<&str>) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        trace!("Creating new Store at path: {:?}", root_path.as_ref());
        let root_path = root_path.as_ref().to_path_buf();
        tokio_fs::create_dir_all(&root_path).await.map_err(|e| {
            error!(
                "Failed to create store root directory {:?}: {}",
                root_path, e
            );
            e
        })?;
        debug!(
            "Store root directory created or already exists: {:?}",
            root_path
        );

        // Load or create store metadata
        let metadata_path = root_path.join(STORE_METADATA_FILE);
        let store_metadata = if tokio_fs::try_exists(&metadata_path).await.unwrap_or(false) {
            debug!("Loading existing store metadata");
            let content = tokio_fs::read_to_string(&metadata_path).await?;
            serde_json::from_str(&content)?
        }
        else {
            debug!("Creating new store metadata");
            let metadata = StoreMetadata::new();
            let content = serde_json::to_string_pretty(&metadata)?;
            tokio_fs::write(&metadata_path, content).await?;
            metadata
        };

        let now = chrono::Utc::now();
        let mut store = Self {
            root_path,
            signing_key: None,
            created_at: now,
            last_accessed_at: std::sync::RwLock::new(now),
            total_size_bytes: std::sync::atomic::AtomicU64::new(store_metadata.total_size_bytes),
            total_documents: std::sync::atomic::AtomicU64::new(store_metadata.total_documents),
            collection_count: std::sync::atomic::AtomicU64::new(store_metadata.collection_count),
        };
        if let Some(passphrase) = passphrase {
            debug!("Passphrase provided, handling signing key");
            let keys_collection = store.collection(KEYS_COLLECTION).await?;
            if let Some(doc) = keys_collection
                .get_with_verification("signing_key", &crate::VerificationOptions::disabled())
                .await?
            {
                // Load existing signing key
                debug!("Loading existing signing key from store");
                let data = doc.data();
                let encrypted = data["encrypted"].as_str().ok_or_else(|| {
                    error!("Stored signing key document missing 'encrypted' field");
                    SentinelError::StoreCorruption {
                        reason: "stored signing key document missing 'encrypted' field or not a string".to_owned(),
                    }
                })?;
                let salt_hex = data["salt"].as_str().ok_or_else(|| {
                    error!("Stored signing key document missing 'salt' field");
                    SentinelError::StoreCorruption {
                        reason: "stored signing key document missing 'salt' field or not a string".to_owned(),
                    }
                })?;
                let salt = hex::decode(salt_hex).map_err(|err| {
                    error!("Stored signing key salt is not valid hex: {}", err);
                    SentinelError::StoreCorruption {
                        reason: format!("stored signing key salt is not valid hex ({})", err),
                    }
                })?;
                let encryption_key = sentinel_crypto::derive_key_from_passphrase_with_salt(passphrase, &salt).await?;
                let key_bytes = sentinel_crypto::decrypt_data(encrypted, &encryption_key).await?;
                let key_array: [u8; 32] = key_bytes.try_into().map_err(|kb: Vec<u8>| {
                    error!(
                        "Stored signing key has invalid length: {}, expected 32",
                        kb.len()
                    );
                    SentinelError::StoreCorruption {
                        reason: format!(
                            "stored signing key has an invalid length ({}, expected 32)",
                            kb.len()
                        ),
                    }
                })?;
                let signing_key = sentinel_crypto::SigningKey::from_bytes(&key_array);
                store.signing_key = Some(Arc::new(signing_key));
                debug!("Existing signing key loaded successfully");
            }
            else {
                // Generate new signing key and salt
                debug!("Generating new signing key");
                let (salt, encryption_key) = sentinel_crypto::derive_key_from_passphrase(passphrase).await?;
                let signing_key = sentinel_crypto::SigningKeyManager::generate_key();
                let key_bytes = signing_key.to_bytes();
                let encrypted = sentinel_crypto::encrypt_data(&key_bytes, &encryption_key).await?;
                let salt_hex = hex::encode(&salt);
                keys_collection
                    .insert(
                        "signing_key",
                        serde_json::json!({"encrypted": encrypted, "salt": salt_hex}),
                    )
                    .await?;
                store.signing_key = Some(Arc::new(signing_key));
                debug!("New signing key generated and stored");
            }
        }
        trace!("Store created successfully");
        Ok(store)
    }

    /// Returns the creation timestamp of the store.
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }

    /// Returns the last access timestamp of the store.
    pub fn last_accessed_at(&self) -> DateTime<Utc> { *self.last_accessed_at.read().unwrap() }

    /// Returns the total size of all collections in the store in bytes.
    pub fn total_size_bytes(&self) -> u64 {
        self.total_size_bytes
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Returns the total number of documents across all collections in the store.
    pub fn total_documents(&self) -> u64 {
        self.total_documents
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Returns the total number of collections in the store.
    pub fn collection_count(&self) -> u64 {
        self.collection_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Retrieves or creates a collection with the specified name.
    ///
    /// This method provides access to a named collection within the store. If the
    /// collection directory doesn't exist, it will be created automatically under
    /// the `data/` subdirectory of the store's root path.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the collection. This will be used as the directory name under
    ///   `data/`. The name should be filesystem-safe (avoid special characters that are invalid in
    ///   directory names on your target platform).
    ///
    /// # Returns
    ///
    /// * `Result<Collection>` - Returns a `Collection` instance on success, or a `SentinelError`
    ///   if:
    ///   - The collection directory cannot be created due to permission issues
    ///   - The name contains invalid characters for the filesystem
    ///   - I/O errors occur during directory creation
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sentinel_dbms::Store;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = Store::new("/var/lib/sentinel", None).await?;
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
    /// - Calling this method multiple times with the same name returns separate `Collection`
    ///   instances pointing to the same directory
    /// - The `data/` subdirectory is created automatically on first collection access
    /// - Collections are not cached; each call creates a new `Collection` instance
    /// - No validation is performed on the collection name beyond filesystem constraints
    pub async fn collection(&self, name: &str) -> Result<Collection> {
        trace!("Accessing collection: {}", name);
        validate_collection_name(name)?;
        let path = self.root_path.join(DATA_DIR).join(name);
        tokio_fs::create_dir_all(&path).await.map_err(|e| {
            error!("Failed to create collection directory {:?}: {}", path, e);
            e
        })?;
        debug!("Collection directory ensured: {:?}", path);

        // Load or create collection metadata
        let metadata_path = path.join(COLLECTION_METADATA_FILE);
        let is_new_collection = !tokio_fs::try_exists(&metadata_path).await.unwrap_or(false);
        let metadata = if is_new_collection {
            debug!("Creating new collection metadata for {}", name);
            let metadata = CollectionMetadata::new(name.to_string());
            let content = serde_json::to_string_pretty(&metadata)?;
            tokio_fs::write(&metadata_path, content).await?;
            metadata
        }
        else {
            debug!("Loading existing collection metadata for {}", name);
            let content = tokio_fs::read_to_string(&metadata_path).await?;
            serde_json::from_str(&content)?
        };

        // If this is a new collection, update store metadata
        if is_new_collection {
            self.collection_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.save_store_metadata().await?;
        }

        // Create WAL manager with config from metadata
        let wal_path = path.join(WAL_DIR).join(WAL_FILE);
        let wal_config = metadata.wal_config;
        let wal_manager = Some(Arc::new(
            WalManager::new(wal_path, wal_config.into()).await?,
        ));

        trace!("Collection '{}' accessed successfully", name);
        let now = chrono::Utc::now();

        // Update store metadata
        *self.last_accessed_at.write().unwrap() = now;

        Ok(Collection {
            path,
            signing_key: self.signing_key.clone(),
            wal_manager,
            created_at: now,
            updated_at: std::sync::RwLock::new(now),
            last_checkpoint_at: std::sync::RwLock::new(None),
            total_documents: std::sync::atomic::AtomicU64::new(metadata.document_count),
            total_size_bytes: std::sync::atomic::AtomicU64::new(metadata.total_size_bytes),
        })
    }

    /// Returns the root path of the store.
    ///
    /// This method provides access to the root directory path where the store
    /// is located. This is useful for operations that need to access store-level
    /// metadata files.
    ///
    /// # Returns
    ///
    /// Returns a reference to the `PathBuf` containing the store's root path.
    pub fn root_path(&self) -> &PathBuf { &self.root_path }

    /// Deletes a collection and all its documents.
    ///
    /// This method removes the entire collection directory and all documents within it.
    /// The operation is permanent and cannot be undone. If the collection doesn't exist,
    /// the operation succeeds silently (idempotent).
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the collection to delete
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `SentinelError` if the operation fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_dbms::Store;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = Store::new("/path/to/data", None).await?;
    ///
    /// // Create a collection
    /// let collection = store.collection("temp_collection").await?;
    ///
    /// // ... use collection ...
    ///
    /// // Delete the collection
    /// store.delete_collection("temp_collection").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_collection(&self, name: &str) -> Result<()> {
        trace!("Deleting collection: {}", name);
        validate_collection_name(name)?;
        let path = self.root_path.join("data").join(name);

        // Check if collection exists
        if !path.exists() {
            debug!("Collection '{}' does not exist, nothing to delete", name);
            return Ok(());
        }

        // Load collection metadata to get document count and size before deletion
        let metadata_path = path.join(COLLECTION_METADATA_FILE);
        let collection_metadata = if tokio_fs::try_exists(&metadata_path).await.unwrap_or(false) {
            let content = tokio_fs::read_to_string(&metadata_path).await?;
            Some(serde_json::from_str::<CollectionMetadata>(&content)?)
        }
        else {
            None
        };

        // Remove the entire directory
        tokio_fs::remove_dir_all(&path).await.map_err(|e| {
            error!("Failed to delete collection directory {:?}: {}", path, e);
            e
        })?;

        debug!("Collection '{}' deleted successfully", name);

        // Update store metadata
        *self.last_accessed_at.write().unwrap() = chrono::Utc::now();
        self.collection_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        if let Some(metadata) = collection_metadata {
            self.total_documents.fetch_sub(
                metadata.document_count,
                std::sync::atomic::Ordering::Relaxed,
            );
            self.total_size_bytes.fetch_sub(
                metadata.total_size_bytes,
                std::sync::atomic::Ordering::Relaxed,
            );
        }
        self.save_store_metadata().await?;

        Ok(())
    }

    /// This method returns a list of all collection names that exist in the store.
    /// The names are returned in no particular order.
    ///
    /// # Returns
    ///
    /// Returns a `Vec<String>` containing the names of all collections.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_dbms::Store;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = Store::new("/path/to/data", None).await?;
    ///
    /// // Create some collections
    /// store.collection("users").await?;
    /// store.collection("products").await?;
    ///
    /// // List all collections
    /// let collections = store.list_collections().await?;
    /// assert!(collections.contains(&"users".to_string()));
    /// assert!(collections.contains(&"products".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_collections(&self) -> Result<Vec<String>> {
        trace!("Listing collections");
        let data_path = self.root_path.join("data");

        // Ensure data directory exists
        tokio_fs::create_dir_all(&data_path).await.map_err(|e| {
            error!("Failed to create data directory {:?}: {}", data_path, e);
            e
        })?;

        // Read directory entries
        let mut entries = tokio_fs::read_dir(&data_path).await.map_err(|e| {
            error!("Failed to read data directory {:?}: {}", data_path, e);
            e
        })?;

        let mut collections = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            error!("Failed to read directory entry: {}", e);
            e
        })? {
            if entry
                .file_type()
                .await
                .map_err(|e| {
                    error!("Failed to get file type for entry: {}", e);
                    e
                })?
                .is_dir() &&
                let Some(name) = entry.file_name().to_str()
            {
                collections.push(name.to_owned());
            }
        }

        debug!("Found {} collections", collections.len());
        Ok(collections)
    }

    /// Saves the store metadata to disk.
    ///
    /// This method updates the store's metadata file (.store.json) with the current
    /// values of total_documents, total_size_bytes, and collection_count.
    async fn save_store_metadata(&self) -> Result<()> {
        use sentinel_wal::StoreWalConfig;

        let metadata_path = self.root_path.join(STORE_METADATA_FILE);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let metadata = StoreMetadata {
            version:          META_SENTINEL_VERSION,
            created_at:       self.created_at.timestamp() as u64,
            updated_at:       now,
            collection_count: self
                .collection_count
                .load(std::sync::atomic::Ordering::Relaxed),
            total_documents:  self
                .total_documents
                .load(std::sync::atomic::Ordering::Relaxed),
            total_size_bytes: self
                .total_size_bytes
                .load(std::sync::atomic::Ordering::Relaxed),
            wal_config:       StoreWalConfig::default(),
        };

        let content = serde_json::to_string_pretty(&metadata).map_err(|e| {
            error!("Failed to serialize store metadata: {}", e);
            e
        })?;

        tokio_fs::write(&metadata_path, content)
            .await
            .map_err(|e| {
                error!(
                    "Failed to write store metadata to {:?}: {}",
                    metadata_path, e
                );
                e
            })?;

        debug!("Store metadata saved successfully");
        Ok(())
    }

    pub fn set_signing_key(&mut self, key: sentinel_crypto::SigningKey) { self.signing_key = Some(Arc::new(key)); }
}

/// Validates that a collection name is filesystem-safe across all platforms.
///
/// # Rules
/// - Must not be empty
/// - Must not contain path separators (`/` or `\`)
/// - Must not contain control characters (0x00-0x1F, 0x7F)
/// - Must not be a Windows reserved name (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
/// - Must not start with a dot (.) to avoid hidden directories
/// - Must only contain alphanumeric characters, underscores (_), hyphens (-), and dots (.)
/// - Must not end with a dot or space (Windows limitation)
///
/// # Parameters
/// - `name`: The collection name to validate
///
/// # Returns
/// - `Ok(())` if the name is valid
/// - `Err(SentinelError::InvalidCollectionName)` if the name is invalid
///
/// # Examples
/// ```no_run
/// # use sentinel_dbms::{Store, SentinelError};
/// # use std::path::Path;
/// # async fn example() -> Result<(), SentinelError> {
/// let store = Store::new(Path::new("/tmp/test"), None).await?;
///
/// // Valid names
/// assert!(store.collection("users").await.is_ok());
/// assert!(store.collection("user_data").await.is_ok());
/// assert!(store.collection("data-2024").await.is_ok());
/// assert!(store.collection("test_collection_123").await.is_ok());
///
/// // Invalid names
/// assert!(store.collection("").await.is_err());
/// assert!(store.collection(".hidden").await.is_err());
/// assert!(store.collection("path/traversal").await.is_err());
/// assert!(store.collection("CON").await.is_err());
/// # Ok(())
/// # }
/// ```
fn validate_collection_name(name: &str) -> Result<()> {
    trace!("Validating collection name: {}", name);
    // Check if name is empty
    if name.is_empty() {
        debug!("Collection name is empty");
        return Err(SentinelError::InvalidCollectionName {
            name: name.to_owned(),
        });
    }

    // Check if name starts with a dot (hidden directory)
    if name.starts_with('.') && name != ".keys" {
        debug!("Collection name starts with dot and is not .keys: {}", name);
        return Err(SentinelError::InvalidCollectionName {
            name: name.to_owned(),
        });
    }

    // Check if name ends with a dot or space (Windows limitation)
    if name.ends_with('.') || name.ends_with(' ') {
        warn!("Collection name ends with dot or space: {}", name);
        return Err(SentinelError::InvalidCollectionName {
            name: name.to_owned(),
        });
    }

    // Check for valid characters
    if !is_valid_name_chars(name) {
        debug!("Collection name contains invalid characters: {}", name);
        return Err(SentinelError::InvalidCollectionName {
            name: name.to_owned(),
        });
    }

    // Check for Windows reserved names
    if is_reserved_name(name) {
        debug!("Collection name is a reserved name: {}", name);
        return Err(SentinelError::InvalidCollectionName {
            name: name.to_owned(),
        });
    }

    trace!("Collection name '{}' is valid", name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn test_store_new_creates_directory() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");

        let _store = Store::new(&store_path, None).await.unwrap();
        assert!(store_path.exists());
        assert!(store_path.is_dir());
    }

    #[tokio::test]
    async fn test_store_new_with_existing_directory() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path();

        // Directory already exists
        let _store = Store::new(&store_path, None).await.unwrap();
        assert!(store_path.exists());
    }

    #[tokio::test]
    async fn test_store_collection_creates_subdirectory() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        let collection = store.collection("users").await.unwrap();
        assert!(collection.path.exists());
        assert!(collection.path.is_dir());
        assert_eq!(collection.name(), "users");
    }

    #[tokio::test]
    async fn test_store_collection_with_valid_special_characters() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Test valid names with underscores, hyphens, and dots
        let collection = store.collection("user_data-123").await.unwrap();
        assert!(collection.path.exists());
        assert_eq!(collection.name(), "user_data-123");

        let collection2 = store.collection("test.collection").await.unwrap();
        assert!(collection2.path.exists());
        assert_eq!(collection2.name(), "test.collection");

        let collection3 = store.collection("data_2024-v1.0").await.unwrap();
        assert!(collection3.path.exists());
        assert_eq!(collection3.name(), "data_2024-v1.0");
    }

    #[tokio::test]
    async fn test_store_collection_multiple_calls() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        let coll1 = store.collection("users").await.unwrap();
        let coll2 = store.collection("users").await.unwrap();

        assert_eq!(coll1.name(), coll2.name());
        assert_eq!(coll1.path, coll2.path);
    }

    #[tokio::test]
    async fn test_store_collection_invalid_empty_name() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        let result = store.collection("").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));
    }

    #[tokio::test]
    async fn test_store_collection_invalid_path_separator() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Forward slash
        let result = store.collection("path/traversal").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));

        // Backslash
        let result = store.collection("path\\traversal").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));
    }

    #[tokio::test]
    async fn test_store_collection_invalid_hidden_name() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        let result = store.collection(".hidden").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));
    }

    #[tokio::test]
    async fn test_store_collection_invalid_windows_reserved_names() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        let reserved_names = vec!["CON", "PRN", "AUX", "NUL", "COM1", "LPT1"];
        for name in reserved_names {
            let result = store.collection(name).await;
            assert!(result.is_err(), "Expected '{}' to be invalid", name);
            assert!(matches!(
                result.unwrap_err(),
                SentinelError::InvalidCollectionName { .. }
            ));

            // Test lowercase version
            let result = store.collection(&name.to_lowercase()).await;
            assert!(
                result.is_err(),
                "Expected '{}' to be invalid",
                name.to_lowercase()
            );
            assert!(matches!(
                result.unwrap_err(),
                SentinelError::InvalidCollectionName { .. }
            ));
        }
    }

    #[tokio::test]
    async fn test_store_collection_invalid_control_characters() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Test null byte
        let result = store.collection("test\0name").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));

        // Test other control characters
        let result = store.collection("test\x01name").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));
    }

    #[tokio::test]
    async fn test_store_collection_invalid_special_characters() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        let invalid_chars = vec!["<", ">", ":", "\"", "|", "?", "*"];
        for ch in invalid_chars {
            let name = format!("test{}name", ch);
            let result = store.collection(&name).await;
            assert!(result.is_err(), "Expected name with '{}' to be invalid", ch);
            assert!(matches!(
                result.unwrap_err(),
                SentinelError::InvalidCollectionName { .. }
            ));
        }
    }

    #[tokio::test]
    async fn test_store_collection_invalid_trailing_dot_or_space() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Trailing dot
        let result = store.collection("test.").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));

        // Trailing space
        let result = store.collection("test ").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));
    }

    #[tokio::test]
    async fn test_store_collection_valid_edge_cases() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Single character
        let collection = store.collection("a").await.unwrap();
        assert_eq!(collection.name(), "a");

        // Numbers only
        let collection = store.collection("123").await.unwrap();
        assert_eq!(collection.name(), "123");

        // Max length typical name
        let long_name = "a".repeat(255);
        let collection = store.collection(&long_name).await.unwrap();
        assert_eq!(collection.name(), long_name);
    }

    #[tokio::test]
    async fn test_store_new_with_passphrase() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();
        // Should have created signing key
        assert!(store.signing_key.is_some());
    }

    #[tokio::test]
    async fn test_store_new_with_passphrase_load_existing() {
        let temp_dir = tempdir().unwrap();
        // Create first store with passphrase
        let store1 = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();
        let key1 = store1.signing_key.as_ref().unwrap().clone();

        // Create second store with same passphrase, should load existing key
        let store2 = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();
        let key2 = store2.signing_key.as_ref().unwrap().clone();

        // Should be the same key
        assert_eq!(key1.to_bytes(), key2.to_bytes());
    }

    #[tokio::test]
    async fn test_store_new_with_corrupted_keys() {
        let temp_dir = tempdir().unwrap();
        // First create a store with passphrase to generate keys
        let _store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();

        // Now corrupt the .keys collection by inserting a document with missing fields
        let store2 = Store::new(temp_dir.path(), None).await.unwrap();
        let keys_coll = store2.collection(".keys").await.unwrap();
        // Insert corrupted document
        let corrupted_data = serde_json::json!({
            "salt": "invalid_salt",
            // missing "encrypted"
        });
        keys_coll
            .insert("signing_key", corrupted_data)
            .await
            .unwrap();

        // Now try to create a new store with passphrase, should fail due to corruption
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_new_with_invalid_salt_hex() {
        let temp_dir = tempdir().unwrap();
        // First create a store with passphrase to generate keys
        let _store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();

        // Corrupt the salt to invalid hex
        let store2 = Store::new(temp_dir.path(), None).await.unwrap();
        let keys_coll = store2.collection(".keys").await.unwrap();
        let doc = keys_coll
            .get_with_verification("signing_key", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        let mut data = doc.data().clone();
        data["salt"] = serde_json::Value::String("invalid_hex".to_string());
        keys_coll.insert("signing_key", data).await.unwrap();

        // Try to load
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_new_with_invalid_encrypted_length() {
        let temp_dir = tempdir().unwrap();
        // First create a store with passphrase to generate keys
        let _store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();

        // Corrupt the encrypted to short
        let store2 = Store::new(temp_dir.path(), None).await.unwrap();
        let keys_coll = store2.collection(".keys").await.unwrap();
        let doc = keys_coll
            .get_with_verification("signing_key", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        let mut data = doc.data().clone();
        data["encrypted"] = serde_json::Value::String(hex::encode(&[0u8; 10])); // short
        keys_coll.insert("signing_key", data).await.unwrap();

        // Try to load
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_new_with_corrupted_keys_missing_salt() {
        let temp_dir = tempdir().unwrap();
        // First create a store with passphrase to generate keys
        let _store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();

        // Now corrupt the .keys collection by inserting a document with missing salt
        let store2 = Store::new(temp_dir.path(), None).await.unwrap();
        let keys_coll = store2.collection(".keys").await.unwrap();
        // Insert corrupted document
        let corrupted_data = serde_json::json!({
            "encrypted": "some_encrypted_data"
            // missing "salt"
        });
        keys_coll
            .insert("signing_key", corrupted_data)
            .await
            .unwrap();

        // Now try to create a new store with passphrase, should fail due to missing salt
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_new_with_corrupted_keys_invalid_salt_hex() {
        let temp_dir = tempdir().unwrap();
        // First create a store with passphrase to generate keys
        let _store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();

        // Now corrupt the .keys collection by inserting a document with invalid salt hex
        let store2 = Store::new(temp_dir.path(), None).await.unwrap();
        let keys_coll = store2.collection(".keys").await.unwrap();
        // Insert corrupted document
        let corrupted_data = serde_json::json!({
            "encrypted": "some_encrypted_data",
            "salt": "invalid_hex_salt"
        });
        keys_coll
            .insert("signing_key", corrupted_data)
            .await
            .unwrap();

        // Now try to create a new store with passphrase, should fail due to invalid salt hex
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_new_with_invalid_key_length() {
        // Test line 154-161: invalid key length error
        let temp_dir = tempdir().unwrap();
        // First create a store with passphrase to generate keys
        let _store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();

        // Now corrupt the .keys collection by modifying the encrypted data to have wrong length
        let store2 = Store::new(temp_dir.path(), None).await.unwrap();
        let keys_coll = store2.collection(".keys").await.unwrap();

        // Get the existing document to extract the salt
        let existing_doc = keys_coll
            .get_with_verification("signing_key", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        let salt = existing_doc.data()["salt"].as_str().unwrap();

        // Create encrypted data that will decrypt to wrong length
        let encryption_key =
            sentinel_crypto::derive_key_from_passphrase_with_salt("test_passphrase", &hex::decode(salt).unwrap())
                .await
                .unwrap();
        let wrong_length_bytes = vec![0u8; 16]; // 16 bytes instead of 32
        let encrypted = sentinel_crypto::encrypt_data(&wrong_length_bytes, &encryption_key)
            .await
            .unwrap();

        let corrupted_data = serde_json::json!({
            "encrypted": encrypted,
            "salt": salt
        });
        keys_coll
            .insert("signing_key", corrupted_data)
            .await
            .unwrap();

        // Now try to create a new store with passphrase, should fail due to invalid key length
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_new_creates_root_directory() {
        // Test line 110-117: creating root directory
        let temp_dir = tempdir().unwrap();
        let new_path = temp_dir.path().join("new_store");

        // Ensure path doesn't exist
        assert!(!tokio::fs::metadata(&new_path).await.is_ok());

        // Create store, should create the directory
        let result = Store::new(&new_path, None).await;
        assert!(result.is_ok());

        // Verify directory was created
        assert!(tokio::fs::metadata(&new_path).await.unwrap().is_dir());
    }

    #[tokio::test]
    async fn test_delete_collection_non_existent() {
        // Test lines 304-306: Deleting non-existent collection
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Delete collection that doesn't exist should succeed
        let result = store.delete_collection("non_existent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_collection_success() {
        // Test lines 310-312, 315-316: Successful collection deletion
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Create a collection
        let _collection = store.collection("test_delete").await.unwrap();

        // Verify it exists
        let collections = store.list_collections().await.unwrap();
        assert!(collections.contains(&"test_delete".to_string()));

        // Delete it
        store.delete_collection("test_delete").await.unwrap();

        // Verify it's gone
        let collections = store.list_collections().await.unwrap();
        assert!(!collections.contains(&"test_delete".to_string()));
    }

    #[tokio::test]
    async fn test_list_collections_creates_data_dir() {
        // Test lines 352-354: list_collections creates data directory if needed
        let temp_dir = tempdir().unwrap();
        let new_path = temp_dir.path().join("new_store");
        let store = Store::new(&new_path, None).await.unwrap();

        // Data dir should be created when listing
        let collections = store.list_collections().await.unwrap();
        assert!(collections.is_empty());

        // Verify data directory exists
        let data_path = new_path.join("data");
        assert!(tokio::fs::metadata(&data_path).await.unwrap().is_dir());
    }

    #[tokio::test]
    async fn test_list_collections_with_entries() {
        // Test lines 363-366, 368-371, 376-377: Reading directory entries
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Create multiple collections
        let _c1 = store.collection("collection1").await.unwrap();
        let _c2 = store.collection("collection2").await.unwrap();
        let _c3 = store.collection("collection3").await.unwrap();

        // List and verify
        let collections = store.list_collections().await.unwrap();
        assert_eq!(collections.len(), 3);
        assert!(collections.contains(&"collection1".to_string()));
        assert!(collections.contains(&"collection2".to_string()));
        assert!(collections.contains(&"collection3".to_string()));
    }
}
