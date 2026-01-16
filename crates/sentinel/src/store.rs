use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::fs as tokio_fs;
use tracing::{debug, error, trace, warn};

use crate::{
    validation::{is_reserved_name, is_valid_name_chars},
    Collection,
    Result,
    SentinelError,
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
    root_path:   PathBuf,
    /// The signing key for the store.
    signing_key: Option<Arc<sentinel_crypto::SigningKey>>,
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
        let mut store = Self {
            root_path,
            signing_key: None,
        };
        if let Some(passphrase) = passphrase {
            debug!("Passphrase provided, handling signing key");
            let keys_collection = store.collection(".keys").await?;
            if let Some(doc) = keys_collection.get("signing_key").await? {
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
                let encryption_key = sentinel_crypto::derive_key_from_passphrase_with_salt(passphrase, &salt)?;
                let key_bytes = sentinel_crypto::decrypt_data(encrypted, &encryption_key)?;
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
                let (salt, encryption_key) = sentinel_crypto::derive_key_from_passphrase(passphrase)?;
                let signing_key = sentinel_crypto::SigningKeyManager::generate_key();
                let key_bytes = signing_key.to_bytes();
                let encrypted = sentinel_crypto::encrypt_data(&key_bytes, &encryption_key)?;
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
        let path = self.root_path.join("data").join(name);
        tokio_fs::create_dir_all(&path).await.map_err(|e| {
            error!("Failed to create collection directory {:?}: {}", path, e);
            e
        })?;
        debug!("Collection directory ensured: {:?}", path);
        trace!("Collection '{}' accessed successfully", name);
        Ok(Collection {
            path,
            signing_key: self.signing_key.clone(),
        })
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
        warn!("Collection name is empty");
        return Err(SentinelError::InvalidCollectionName {
            name: name.to_owned(),
        });
    }

    // Check if name starts with a dot (hidden directory)
    if name.starts_with('.') && name != ".keys" {
        warn!("Collection name starts with dot and is not .keys: {}", name);
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
        warn!("Collection name contains invalid characters: {}", name);
        return Err(SentinelError::InvalidCollectionName {
            name: name.to_owned(),
        });
    }

    // Check for Windows reserved names
    if is_reserved_name(name) {
        warn!("Collection name is a reserved name: {}", name);
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
        let doc = keys_coll.get("signing_key").await.unwrap().unwrap();
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
        let doc = keys_coll.get("signing_key").await.unwrap().unwrap();
        let mut data = doc.data().clone();
        data["encrypted"] = serde_json::Value::String(hex::encode(&[0u8; 10])); // short
        keys_coll.insert("signing_key", data).await.unwrap();

        // Try to load
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }
}
