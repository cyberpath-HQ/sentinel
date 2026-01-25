use std::{path::PathBuf, sync::Arc};

use async_stream::stream;
use futures::{StreamExt as _, TryStreamExt as _};
use serde_json::{json, Value};
use tokio::fs as tokio_fs;
use tokio_stream::Stream;
use tracing::{debug, error, trace, warn};
use sentinel_wal::{EntryType, LogEntry, WalDocumentOps, WalManager};

use crate::{
    comparison::compare_values,
    constants::COLLECTION_METADATA_FILE,
    events::{EventEmitter, StoreEvent},
    filtering::matches_filters,
    metadata::CollectionMetadata,
    projection::project_document,
    query::{Aggregation, Filter},
    streaming::stream_document_ids,
    validation::{is_reserved_name, is_valid_document_id_chars},
    Document,
    Result,
    SentinelError,
};

/// A collection represents a namespace for documents in the Sentinel database.
///
/// Collections are backed by filesystem directories, where each document is stored
/// as a JSON file with metadata including version, timestamps, hash, and optional signature.
/// The collection provides CRUD operations (Create, Read, Update, Delete) and advanced
/// querying capabilities with streaming support for memory-efficient handling of large datasets.
///
/// # Structure
///
/// Each collection is stored in a directory with the following structure:
/// - `{collection_name}/` - Root directory for the collection
/// - `{collection_name}/{id}.json` - Individual document files with embedded metadata
/// - `{collection_name}/.deleted/` - Soft-deleted documents (for recovery)
/// - `{collection_name}/.metadata.json` - Collection metadata and indices (future)
///
/// # Streaming Operations
///
/// For memory efficiency with large datasets, operations like `filter()` and `query()`
/// return async streams that process documents one-by-one rather than loading
/// all documents into memory simultaneously.
///
/// # Example
///
/// ```rust
/// use sentinel_dbms::{Store, Collection};
/// use futures::TryStreamExt;
/// use serde_json::json;
///
/// # async fn example() -> sentinel_dbms::Result<()> {
/// // Create a store and get a collection
/// let store = Store::new("/tmp/sentinel", None).await?;
/// let collection = store.collection("users").await?;
///
/// // Insert a document
/// let user_data = json!({
///     "name": "Alice",
///     "email": "alice@example.com",
///     "age": 30
/// });
/// collection.insert("user-123", user_data).await?;
///
/// // Retrieve the document
/// let doc = collection.get("user-123").await?;
/// assert!(doc.is_some());
/// assert_eq!(doc.unwrap().id(), "user-123");
///
/// // Stream all documents matching a predicate
/// let adults = collection.filter(|doc| {
///     doc.data().get("age")
///         .and_then(|v| v.as_i64())
///         .map_or(false, |age| age >= 18)
/// });
/// let adult_docs: Vec<_> = adults.try_collect().await?;
/// assert_eq!(adult_docs.len(), 1);
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
    pub(crate) path:               PathBuf,
    /// The signing key for the collection.
    pub(crate) signing_key:        Option<Arc<sentinel_crypto::SigningKey>>,
    /// The Write-Ahead Log manager for durability.
    pub(crate) wal_manager:        Option<Arc<WalManager>>,
    /// WAL configuration stored in metadata (without temporary overrides).
    pub(crate) stored_wal_config:  sentinel_wal::CollectionWalConfig,
    /// Effective WAL configuration (stored + any temporary overrides).
    pub(crate) wal_config:         sentinel_wal::CollectionWalConfig,
    /// When the collection was created.
    pub(crate) created_at:         chrono::DateTime<chrono::Utc>,
    /// When the collection was last updated.
    pub(crate) updated_at:         std::sync::RwLock<chrono::DateTime<chrono::Utc>>,
    /// When the collection was last checkpointed.
    pub(crate) last_checkpoint_at: std::sync::RwLock<Option<chrono::DateTime<chrono::Utc>>>,
    /// Total number of documents in the collection.
    pub(crate) total_documents:    std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// Total size of all documents in the collection in bytes.
    pub(crate) total_size_bytes:   std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// Event sender for notifying the store of metadata changes.
    pub(crate) event_sender:       Option<tokio::sync::mpsc::UnboundedSender<crate::events::StoreEvent>>,
    /// Background task handle for processing internal events.
    pub(crate) event_task:         Option<tokio::task::JoinHandle<()>>,
}

impl Collection {
    /// Returns the name of the collection.
    pub fn name(&self) -> &str { self.path.file_name().unwrap().to_str().unwrap() }

    /// Returns the creation timestamp of the collection.
    pub fn created_at(&self) -> chrono::DateTime<chrono::Utc> { self.created_at }

    /// Returns the last update timestamp of the collection.
    pub fn updated_at(&self) -> chrono::DateTime<chrono::Utc> { *self.updated_at.read().unwrap() }

    /// Returns the last checkpoint timestamp of the collection, if any.
    pub fn last_checkpoint_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        *self.last_checkpoint_at.read().unwrap()
    }

    /// Returns the total number of documents in the collection.
    pub fn total_documents(&self) -> u64 {
        self.total_documents
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Returns the total size of all documents in the collection in bytes.
    pub fn total_size_bytes(&self) -> u64 {
        self.total_size_bytes
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Returns a reference to the stored WAL configuration for this collection.
    ///
    /// This is the WAL configuration as persisted in the collection metadata,
    /// without any temporary overrides that may be applied at runtime.
    pub fn stored_wal_config(&self) -> &sentinel_wal::CollectionWalConfig { &self.stored_wal_config }

    /// Returns the effective WAL configuration for this collection.
    ///
    /// This includes the stored configuration plus any runtime overrides that
    /// may have been applied when the collection was accessed.
    pub fn wal_config(&self) -> &sentinel_wal::CollectionWalConfig { &self.wal_config }

    /// Saves the current collection metadata to disk.
    ///
    /// This method persists the collection's current state (document count, size, timestamps,
    /// and WAL configuration) to the `.metadata.json` file in the collection directory. This
    /// ensures that metadata remains consistent across restarts and can be used for monitoring
    /// and optimization.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `SentinelError` if the metadata cannot be saved.
    pub async fn save_metadata(&self) -> Result<()> {
        let metadata_path = self.path.join(COLLECTION_METADATA_FILE);

        // Load existing metadata to preserve other fields
        let mut metadata = if tokio_fs::try_exists(&metadata_path).await.unwrap_or(false) {
            let content = tokio_fs::read_to_string(&metadata_path).await?;
            serde_json::from_str(&content)?
        }
        else {
            // Create new metadata if file doesn't exist
            CollectionMetadata::new(self.name().to_string())
        };

        // Update the runtime statistics
        metadata.document_count = self.total_documents();
        metadata.total_size_bytes = self.total_size_bytes();
        metadata.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Update the WAL configuration
        metadata.wal_config = Some(self.stored_wal_config.clone());

        // Save back to disk
        let content = serde_json::to_string_pretty(&metadata)?;
        tokio_fs::write(&metadata_path, content).await?;

        debug!("Collection metadata saved for {}", self.name());
        Ok(())
    }

    /// Flushes any pending metadata changes to disk immediately.
    ///
    /// This method forces a synchronous save of the collection metadata to disk,
    /// bypassing the normal debounced save mechanism. This is useful for tests
    /// and for ensuring data durability when needed.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `SentinelError` if the metadata cannot be saved.
    pub async fn flush_metadata(&self) -> Result<()> { self.save_metadata().await }

    /// Validates a document ID according to filesystem-safe naming rules.
    ///
    /// Document IDs must be filesystem-safe and cannot contain reserved characters
    /// or Windows reserved names. This prevents issues with file operations and
    /// ensures cross-platform compatibility.
    ///
    /// # Arguments
    ///
    /// * `id` - The document ID to validate.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the ID is valid, or a `SentinelError::InvalidDocumentId`
    /// if the ID contains invalid characters or is a reserved name.
    ///
    /// # Validation Rules
    ///
    /// - Must not be empty
    /// - Must not contain path separators (`/` or `\`)
    /// - Must not contain control characters (0x00-0x1F)
    /// - Must not contain Windows reserved characters (`< > : " | ? *`)
    /// - Must not be a Windows reserved name (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
    /// - Must not contain spaces or other filesystem-unsafe characters
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_dbms::Collection;
    ///
    /// // Valid IDs
    /// assert!(Collection::validate_document_id("user-123").is_ok());
    /// assert!(Collection::validate_document_id("my_document").is_ok());
    ///
    /// // Invalid IDs
    /// assert!(Collection::validate_document_id("").is_err()); // empty
    /// assert!(Collection::validate_document_id("path/file").is_err()); // path separator
    /// assert!(Collection::validate_document_id("CON").is_err()); // reserved name
    /// ```
    pub fn validate_document_id(id: &str) -> Result<()> {
        if id.is_empty() {
            return Err(SentinelError::InvalidDocumentId {
                id: id.to_string(),
            });
        }

        // Check for path separators
        if id.contains('/') || id.contains('\\') {
            return Err(SentinelError::InvalidDocumentId {
                id: id.to_string(),
            });
        }

        // Check for control characters
        if id.chars().any(|c| c.is_control()) {
            return Err(SentinelError::InvalidDocumentId {
                id: id.to_string(),
            });
        }

        // Check for Windows reserved characters
        let reserved_chars = ['<', '>', ':', '"', '|', '?', '*'];
        if id.chars().any(|c| reserved_chars.contains(&c)) {
            return Err(SentinelError::InvalidDocumentId {
                id: id.to_string(),
            });
        }

        // Check for Windows reserved names (case-insensitive)
        let reserved_names = [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1",
            "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];
        let upper_id = id.to_uppercase();
        for reserved in &reserved_names {
            if upper_id == *reserved || upper_id.starts_with(&format!("{}.", reserved)) {
                return Err(SentinelError::InvalidDocumentId {
                    id: id.to_string(),
                });
            }
        }

        // Check for other filesystem-unsafe characters
        if !is_valid_document_id_chars(id) {
            return Err(SentinelError::InvalidDocumentId {
                id: id.to_string(),
            });
        }

        Ok(())
    }

    /// Starts the background event processing task for the collection.
    ///
    /// This method spawns an async task that processes internal collection events
    /// such as metadata updates and WAL operations. The task runs in the background
    /// and handles events sent via the event channel.
    ///
    /// The event processor is responsible for:
    /// - Processing document events (insert, update, delete)
    /// - Debounced metadata persistence (every 500ms)
    /// - Coordinating with the store's event system
    ///
    /// # Note
    ///
    /// This method should only be called once during collection initialization.
    /// Multiple calls will replace the previous event task.
    pub fn start_event_processor(&mut self) {
        let mut event_receiver = {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            self.event_sender = Some(tx);
            rx
        };

        // Clone necessary fields for the background task
        let path = self.path.clone();
        let total_documents = self.total_documents.clone();
        let total_size_bytes = self.total_size_bytes.clone();
        let updated_at = std::sync::Arc::new(std::sync::RwLock::new(*self.updated_at.read().unwrap()));

        let task = tokio::spawn(async move {
            // Debouncing: save metadata every 500 milliseconds instead of after every event
            let mut save_interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
            save_interval.tick().await; // First tick completes immediately

            let mut changed = false;

            loop {
                tokio::select! {
                    // Process events
                    event = event_receiver.recv() => {
                        match event {
                            Some(crate::events::StoreEvent::CollectionCreated {
                                ..
                            }) => {
                                // Collection creation is handled by the store
                            },
                            Some(crate::events::StoreEvent::CollectionDeleted {
                                ..
                            }) => {
                                // Collection dropping is handled by the store
                            },
                            Some(crate::events::StoreEvent::DocumentInserted {
                                collection,
                                size_bytes,
                            }) => {
                                tracing::debug!("Processing document inserted event: {} (size: {})", collection, size_bytes);
                                // Update atomic counters asynchronously
                                total_documents.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                total_size_bytes.fetch_add(size_bytes, std::sync::atomic::Ordering::Relaxed);
                                changed = true;
                            },
                            Some(crate::events::StoreEvent::DocumentUpdated {
                                collection,
                                old_size_bytes,
                                new_size_bytes,
                            }) => {
                                tracing::debug!("Processing document updated event: {} (old: {}, new: {})",
                                    collection, old_size_bytes, new_size_bytes);
                                // Update atomic counters asynchronously
                                total_size_bytes.fetch_sub(old_size_bytes, std::sync::atomic::Ordering::Relaxed);
                                total_size_bytes.fetch_add(new_size_bytes, std::sync::atomic::Ordering::Relaxed);
                                changed = true;
                            },
                            Some(crate::events::StoreEvent::DocumentDeleted {
                                collection,
                                size_bytes,
                            }) => {
                                tracing::debug!("Processing document deleted event: {} (size: {})", collection, size_bytes);
                                // Update atomic counters asynchronously
                                total_documents.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                                total_size_bytes.fetch_sub(size_bytes, std::sync::atomic::Ordering::Relaxed);
                                changed = true;
                            },
                            None => {
                                // Channel closed, exit
                                break;
                            },
                        }
                    }

                    // Periodic metadata save
                    _ = save_interval.tick() => {
                        if changed {
                            // Update the updated_at timestamp (read lock and update)
                            let now = chrono::Utc::now();
                            *updated_at.write().unwrap() = now;

                            // Load current values from atomic counters
                            let document_count = total_documents.load(std::sync::atomic::Ordering::Relaxed);
                            let size_bytes = total_size_bytes.load(std::sync::atomic::Ordering::Relaxed);

                            // Load existing metadata to preserve other fields
                            let metadata_path = path.join(crate::constants::COLLECTION_METADATA_FILE);
                            let mut metadata = if tokio::fs::try_exists(&metadata_path).await.unwrap_or(false) {
                                match tokio::fs::read_to_string(&metadata_path).await {
                                    Ok(content) => match serde_json::from_str(&content) {
                                        Ok(m) => m,
                                        Err(e) => {
                                            tracing::error!("Failed to parse collection metadata: {}", e);
                                            continue;
                                        }
                                    },
                                    Err(e) => {
                                        tracing::error!("Failed to read collection metadata: {}", e);
                                        continue;
                                    }
                                }
                            } else {
                                tracing::warn!("Collection metadata file not found, creating new");
                                crate::CollectionMetadata::new(path.file_name().unwrap().to_str().unwrap().to_string())
                            };

                            // Update the runtime statistics
                            metadata.document_count = document_count;
                            metadata.total_size_bytes = size_bytes;
                            metadata.updated_at = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs();

                            // Save back to disk
                            match serde_json::to_string_pretty(&metadata) {
                                Ok(content) => {
                                    if let Err(e) = tokio::fs::write(&metadata_path, content).await {
                                        tracing::error!("Failed to save collection metadata in background task: {}", e);
                                    } else {
                                        tracing::trace!("Collection metadata saved successfully for {:?}", path);
                                        changed = false;
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to serialize collection metadata: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        });

        self.event_task = Some(task);
    }

    /// Emits an event to the store's event system.
    ///
    /// This is an internal method used to notify the store of collection-level
    /// events such as document insertions, updates, and deletions. The events
    /// are sent asynchronously and do not block the calling operation.
    ///
    /// # Arguments
    /// Emits an event to the collection's event sender.
    ///
    /// * `event` - The event to emit to the store.
    pub fn emit_event(&self, event: crate::events::StoreEvent) {
        if let Some(sender) = &self.event_sender {
            if let Err(e) = sender.send(event) {
                warn!("Failed to emit collection event: {}", e);
            }
        }
    }
}
