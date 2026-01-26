use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use tokio::{fs as tokio_fs, sync::mpsc};
use tracing::{debug, error, trace};

use crate::{events::StoreEvent, Result, SentinelError, StoreMetadata, KEYS_COLLECTION, STORE_METADATA_FILE};
use super::{events::start_event_processor, operations::collection_with_config};

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
#[allow(clippy::field_scoped_visibility_modifiers)]
#[derive(Debug)]
pub struct Store {
    /// The root path of the store.
    pub(crate) root_path:         PathBuf,
    /// The signing key for the store.
    pub(crate) signing_key:       Option<Arc<sentinel_crypto::SigningKey>>,
    /// When the store was created.
    pub(crate) created_at:        chrono::DateTime<chrono::Utc>,
    /// When the store was last accessed.
    pub(crate) last_accessed_at:  std::sync::RwLock<chrono::DateTime<chrono::Utc>>,
    /// Total size of all collections in bytes.
    pub(crate) total_size_bytes:  std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// Total number of documents across all collections.
    pub(crate) total_documents:   std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// Total number of collections.
    pub(crate) collection_count:  std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// WAL configuration for the store (effective/runtime config).
    pub(crate) wal_config:        sentinel_wal::StoreWalConfig,
    /// WAL configuration for the store (stored/persisted config).
    pub(crate) stored_wal_config: sentinel_wal::StoreWalConfig,
    /// Channel receiver for events from collections.
    pub(crate) event_receiver:    Option<mpsc::UnboundedReceiver<StoreEvent>>,
    /// Channel sender for collections to emit events.
    pub(crate) event_sender:      mpsc::UnboundedSender<StoreEvent>,
    /// Background task handle for processing events.
    pub(crate) event_task:        Option<tokio::task::JoinHandle<()>>,
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
    #[deprecated(
        since = "2.0.2",
        note = "Please use new_with_config to specify WAL configuration"
    )]
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

        // Create event channel for collection synchronization
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let mut store = Self {
            root_path,
            signing_key: None,
            created_at: now,
            last_accessed_at: std::sync::RwLock::new(now),
            total_size_bytes: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(
                store_metadata.total_size_bytes,
            )),
            total_documents: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(
                store_metadata.total_documents,
            )),
            collection_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(
                store_metadata.collection_count,
            )),
            stored_wal_config: store_metadata.wal_config.clone(),
            wal_config: store_metadata.wal_config.clone(),
            event_receiver: Some(event_receiver),
            event_sender,
            event_task: None,
        };
        if let Some(passphrase) = passphrase {
            debug!("Passphrase provided, handling signing key");
            let keys_collection = collection_with_config(&store, KEYS_COLLECTION, None).await?;
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

        // Start background event processing task
        start_event_processor(&mut store);

        Ok(store)
    }

    /// Creates a new `Store` instance at the specified root path with custom WAL configuration.
    ///
    /// This method initializes the store by creating the root directory if it doesn't
    /// exist and applies the provided WAL configuration. It does not create the `data/`
    /// subdirectory until collections are accessed.
    ///
    /// # Parameters
    ///
    /// * `root_path` - The filesystem path where the store will be created. This can be any type
    ///   that implements `AsRef<Path>`, including `&str`, `String`, `Path`, and `PathBuf`.
    /// * `passphrase` - Optional passphrase for encrypting the signing key
    /// * `wal_config` - Custom WAL configuration for the store
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
    /// use sentinel_wal::StoreWalConfig;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let wal_config = StoreWalConfig::default();
    /// let store =
    ///     Store::new_with_config("/var/lib/sentinel", None, wal_config).await?;
    /// # Ok(())
    /// # }
    /// ```
#[allow(clippy::cognitive_complexity, reason = "complex initialization logic")]
    pub async fn new_with_config<P>(
        root_path: P,
        passphrase: Option<&str>,
        wal_config: sentinel_wal::StoreWalConfig,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        trace!(
            "Creating new Store at path: {:?} with custom WAL config",
            root_path.as_ref()
        );
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

        // Load or create store metadata with custom WAL config
        let metadata_path = root_path.join(STORE_METADATA_FILE);
        let store_metadata = if tokio_fs::try_exists(&metadata_path).await.unwrap_or(false) {
            debug!("Loading existing store metadata");
            let mut metadata: StoreMetadata = {
                let content = tokio_fs::read_to_string(&metadata_path).await?;
                serde_json::from_str(&content)?
            };
            // Update WAL config if store already exists
            metadata.wal_config = wal_config;
            let content = serde_json::to_string_pretty(&metadata)?;
            tokio_fs::write(&metadata_path, content).await?;
            metadata
        }
        else {
            debug!("Creating new store metadata with custom WAL config");
            let mut metadata = StoreMetadata::new();
            metadata.wal_config = wal_config;
            let content = serde_json::to_string_pretty(&metadata)?;
            tokio_fs::write(&metadata_path, content).await?;
            metadata
        };

        let now = chrono::Utc::now();

        // Create event channel for collection synchronization
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let mut store = Self {
            root_path,
            signing_key: None,
            created_at: now,
            last_accessed_at: std::sync::RwLock::new(now),
            total_size_bytes: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(
                store_metadata.total_size_bytes,
            )),
            total_documents: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(
                store_metadata.total_documents,
            )),
            collection_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(
                store_metadata.collection_count,
            )),
            wal_config: store_metadata.wal_config.clone(),
            stored_wal_config: store_metadata.wal_config,
            event_receiver: Some(event_receiver),
            event_sender,
            event_task: None,
        };
        if let Some(passphrase) = passphrase {
            debug!("Passphrase provided, handling signing key");
            let keys_collection = collection_with_config(&store, KEYS_COLLECTION, None).await?;
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

        // Start background event processing task
        start_event_processor(&mut store);

        Ok(store)
    }

    /// Returns the creation timestamp of the store.
    pub const fn created_at(&self) -> DateTime<Utc> { self.created_at }

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

    /// Returns the root path of the store.
    ///
    /// This method provides access to the root directory path where the store
    /// is located. This is useful for operations that need to access store-level
    /// metadata files.
    ///
    /// # Returns
    ///
    /// Returns a reference to the `PathBuf` containing the store's root path.
    pub const fn root_path(&self) -> &PathBuf { &self.root_path }

    /// Returns a clone of the event sender for collections to emit events.
    pub(crate) fn event_sender(&self) -> mpsc::UnboundedSender<StoreEvent> { self.event_sender.clone() }
}

impl Drop for Store {
    fn drop(&mut self) {
        // Close the event channel to signal the background task to stop
        // The receiver will be dropped when the task finishes
        if let Some(task) = self.event_task.take() {
            // We can't await here, but the task will be aborted when the runtime shuts down
            task.abort();
        }
    }
}
