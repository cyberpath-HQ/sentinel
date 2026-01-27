use std::sync::Arc;

use tokio::fs as tokio_fs;
use tracing::{debug, error, trace};
use sentinel_wal::WalManager;

use crate::{
    events::StoreEvent,
    Collection,
    CollectionMetadata,
    Result,
    COLLECTION_METADATA_FILE,
    DATA_DIR,
    WAL_DIR,
    WAL_FILE,
};
use super::{stor::Store, validation::validate_collection_name};

/// Retrieves or creates a collection with the specified name and custom WAL configuration
/// overrides.
///
/// This is an internal function used by the Store impl. Use Store::collection_with_config instead.
pub async fn collection_with_config(
    store: &Store,
    name: &str,
    wal_overrides: Option<sentinel_wal::CollectionWalConfigOverrides>,
) -> Result<Collection> {
    trace!("Accessing collection: {} with custom WAL config", name);
    validate_collection_name(name)?;
    let path = store.root_path.join(DATA_DIR).join(name);
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
        let mut metadata = CollectionMetadata::new(name.to_owned());
        // For new collections, if overrides are provided, create a config with overrides applied to
        // defaults
        if let Some(overrides) = wal_overrides.as_ref() {
            let base_config = store
                .wal_config
                .collection_configs
                .get(name)
                .cloned()
                .unwrap_or_else(|| store.wal_config.default_collection_config.clone());
            let merged_config = base_config.apply_overrides(overrides);
            metadata.wal_config = Some(merged_config);
        }
        let content = serde_json::to_string_pretty(&metadata)?;
        tokio_fs::write(&metadata_path, content).await?;
        metadata
    }
    else {
        debug!("Loading existing collection metadata for {}", name);
        let content = tokio_fs::read_to_string(&metadata_path).await?;
        let mut metadata: CollectionMetadata = serde_json::from_str(&content)?;
        // For existing collections, conditionally update metadata if persist_overrides is true
        if let Some(overrides) = wal_overrides.as_ref() &&
            overrides.persist_overrides
        {
            let base_config = metadata.wal_config.unwrap_or_else(|| {
                store
                    .wal_config
                    .collection_configs
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| store.wal_config.default_collection_config.clone())
            });
            let merged_config = base_config.apply_overrides(overrides);
            metadata.wal_config = Some(merged_config);
            let content = serde_json::to_string_pretty(&metadata)?;
            tokio_fs::write(&metadata_path, content).await?;
        }
        metadata
    };

    // If this is a new collection, emit event (metadata will be saved by event handler)
    if is_new_collection {
        // Emit collection created event
        let event = StoreEvent::CollectionCreated {
            name: name.to_owned(),
        };
        let _ = store.event_sender.send(event).ok();
    }

    // Get collection WAL config: use metadata's config, or provided config, or fall back to
    // store-derived
    let stored_wal_config = metadata.wal_config.clone().unwrap_or_else(|| {
        store
            .wal_config
            .collection_configs
            .get(name)
            .cloned()
            .unwrap_or_else(|| store.wal_config.default_collection_config.clone())
    });

    let mut collection_wal_config = stored_wal_config.clone();

    // Apply overrides if provided
    if let Some(overrides) = wal_overrides {
        collection_wal_config = collection_wal_config.apply_overrides(&overrides);
    }

    // Create WAL manager with collection config
    let wal_path = path.join(WAL_DIR).join(WAL_FILE);
    let wal_manager = Some(Arc::new(
        WalManager::new(wal_path, collection_wal_config.clone().into()).await?,
    ));

    trace!("Collection '{}' accessed successfully", name);
    let now = chrono::Utc::now();

    // Update store metadata
    *store.last_accessed_at.write().unwrap() = now;

    let mut collection = Collection {
        path,
        signing_key: store.signing_key.clone(),
        wal_manager,
        wal_config: collection_wal_config,
        stored_wal_config,
        created_at: now,
        updated_at: std::sync::RwLock::new(now),
        last_checkpoint_at: std::sync::RwLock::new(None),
        total_documents: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(metadata.document_count)),
        total_size_bytes: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(metadata.total_size_bytes)),
        event_sender: Some(store.event_sender.clone()),
        event_task: None,
        recovery_mode: std::sync::atomic::AtomicBool::new(false),
    };
    collection.start_event_processor();
    Ok(collection)
}

#[allow(
    clippy::multiple_inherent_impl,
    reason = "multiple impl blocks for Store are intentional for organization"
)]
impl Store {
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
    #[deprecated(
        since = "2.0.2",
        note = "Please use collection_with_config to specify WAL configuration"
    )]
    pub async fn collection(&self, name: &str) -> Result<Collection> { collection_with_config(self, name, None).await }

    /// Retrieves or creates a collection with the specified name and custom WAL configuration
    /// overrides.
    ///
    /// This method provides access to a named collection within the store with custom WAL settings
    /// that override the stored or default configuration. If the collection directory doesn't
    /// exist, it will be created automatically under the `data/` subdirectory of the store's
    /// root path.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the collection. This will be used as the directory name under
    ///   `data/`. The name should be filesystem-safe.
    /// * `wal_overrides` - Optional WAL configuration overrides for this collection
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
    /// use sentinel_wal::CollectionWalConfigOverrides;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = Store::new("/var/lib/sentinel", None).await?;
    /// let wal_overrides = CollectionWalConfigOverrides {
    ///     write_mode: Some(sentinel_wal::WalFailureMode::Warn),
    ///     ..Default::default()
    /// };
    ///
    /// // Access a users collection with WAL overrides
    /// let users = store.collection_with_config("users", Some(wal_overrides)).await?;
    ///
    /// // Insert a document into the collection
    /// users.insert("user-123", json!({
    ///     "name": "Alice",
    ///     "email": "alice@example.com"
    /// })).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn collection_with_config(
        &self,
        name: &str,
        wal_overrides: Option<sentinel_wal::CollectionWalConfigOverrides>,
    ) -> Result<Collection> {
        collection_with_config(self, name, wal_overrides).await
    }

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
        if let Some(metadata) = collection_metadata {
            // Emit collection deleted event (metadata will be saved by event handler)
            let event = StoreEvent::CollectionDeleted {
                name:             name.to_owned(),
                document_count:   metadata.document_count,
                total_size_bytes: metadata.total_size_bytes,
            };
            drop(self.event_sender.send(event));
        }

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

    pub fn set_signing_key(&mut self, key: sentinel_crypto::SigningKey) { self.signing_key = Some(Arc::new(key)); }
}
