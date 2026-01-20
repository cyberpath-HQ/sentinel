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

#[allow(
    unexpected_cfgs,
    reason = "tarpaulin_include is set by code coverage tool"
)]
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

    /// Saves the current collection metadata to disk.
    ///
    /// This method persists the collection's current state (document count, size, timestamps)
    /// to the `.metadata.json` file in the collection directory. This ensures that metadata
    /// remains consistent across restarts and can be used for monitoring and optimization.
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

        // Save back to disk
        let content = serde_json::to_string_pretty(&metadata)?;
        tokio_fs::write(&metadata_path, content).await?;

        debug!("Collection metadata saved for {}", self.name());
        Ok(())
    }

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
    /// - Processing metadata update events
    /// - Handling WAL checkpoint operations
    /// - Coordinating with the store's event system
    ///
    /// # Note
    ///
    /// This method should only be called once during collection initialization.
    /// Multiple calls will replace the previous event task.
    pub fn start_event_processor(&mut self) {
        let event_receiver = {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            self.event_sender = Some(tx);
            rx
        };

        let task = tokio::spawn(async move {
            let mut receiver = event_receiver;
            while let Some(event) = receiver.recv().await {
                match event {
                    StoreEvent::CollectionCreated {
                        ..
                    } => {
                        // Collection creation is handled by the store
                    },
                    StoreEvent::CollectionDeleted {
                        ..
                    } => {
                        // Collection dropping is handled by the store
                    },
                    StoreEvent::DocumentInserted {
                        ..
                    } => {
                        // Document insertion metadata is handled inline
                    },
                    StoreEvent::DocumentUpdated {
                        ..
                    } => {
                        // Document update metadata is handled inline
                    },
                    StoreEvent::DocumentDeleted {
                        ..
                    } => {
                        // Document deletion metadata is handled inline
                    },
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
