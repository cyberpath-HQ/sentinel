use std::{path::PathBuf, sync::Arc};

use async_stream::stream;
use futures::{StreamExt as _, TryStreamExt as _};
use serde_json::Value;
use tokio::fs as tokio_fs;
use tokio_stream::Stream;
use tracing::{debug, error, trace, warn};

use crate::{
    comparison::compare_values,
    filtering::matches_filters,
    projection::project_document,
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::field_scoped_visibility_modifiers,
    reason = "fields need to be pub(crate) for internal access"
)]
pub struct Collection {
    /// The filesystem path to the collection directory.
    pub(crate) path:        PathBuf,
    /// The signing key for the collection.
    pub(crate) signing_key: Option<Arc<sentinel_crypto::SigningKey>>,
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
    /// use sentinel_dbms::{Store, Collection};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
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
        trace!("Inserting document with id: {}", id);
        validate_document_id(id)?;
        let file_path = self.path.join(format!("{}.json", id));

        #[allow(clippy::pattern_type_mismatch, reason = "false positive")]
        let doc = if let Some(key) = &self.signing_key {
            debug!("Creating signed document for id: {}", id);
            Document::new(id.to_owned(), data, key).await?
        }
        else {
            debug!("Creating unsigned document for id: {}", id);
            Document::new_without_signature(id.to_owned(), data).await?
        };
        let json = serde_json::to_string_pretty(&doc).map_err(|e| {
            error!("Failed to serialize document {} to JSON: {}", id, e);
            e
        })?;
        tokio_fs::write(&file_path, json).await.map_err(|e| {
            error!(
                "Failed to write document {} to file {:?}: {}",
                id, file_path, e
            );
            e
        })?;
        debug!("Document {} inserted successfully", id);
        Ok(())
    }

    /// Retrieves a document from the collection by its ID.
    ///
    /// Reads the JSON file corresponding to the given ID and deserializes it into
    /// a `Document` struct. If the document doesn't exist, returns `None`.
    ///
    /// By default, this method verifies both hash and signature with strict mode.
    /// Use `get_with_verification()` to customize verification behavior.
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
    /// use sentinel_dbms::{Store, Collection};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert a document first
    /// collection.insert("user-123", json!({"name": "Alice"})).await?;
    ///
    /// // Retrieve the document (with verification enabled by default)
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
        self.get_with_verification(id, &crate::VerificationOptions::default())
            .await
    }

    /// Retrieves a document from the collection by its ID with custom verification options.
    ///
    /// Reads the JSON file corresponding to the given ID and deserializes it into
    /// a `Document` struct. If the document doesn't exist, returns `None`.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the document to retrieve.
    /// * `options` - Verification options controlling hash and signature verification.
    ///
    /// # Returns
    ///
    /// Returns:
    /// - `Ok(Some(Document))` if the document exists and was successfully read
    /// - `Ok(None)` if the document doesn't exist (file not found)
    /// - `Err(SentinelError)` if there was an error reading, parsing, or verifying the document
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::{Store, Collection, VerificationMode, VerificationOptions};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert a document first
    /// collection.insert("user-123", json!({"name": "Alice"})).await?;
    ///
    /// // Retrieve with warning mode instead of strict
    /// let options = VerificationOptions {
    ///     verify_signature: true,
    ///     verify_hash: true,
    ///     signature_verification_mode: VerificationMode::Warn,
    ///     hash_verification_mode: VerificationMode::Warn,
    /// };
    /// let doc = collection.get_with_verification("user-123", &options).await?;
    /// assert!(doc.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_with_verification(
        &self,
        id: &str,
        options: &crate::VerificationOptions,
    ) -> Result<Option<Document>> {
        trace!(
            "Retrieving document with id: {} (verification enabled: {})",
            id,
            options.verify_signature || options.verify_hash
        );
        validate_document_id(id)?;
        let file_path = self.path.join(format!("{}.json", id));
        match tokio_fs::read_to_string(&file_path).await {
            Ok(content) => {
                debug!("Document {} found, parsing JSON", id);
                let mut doc: Document = serde_json::from_str(&content).map_err(|e| {
                    error!("Failed to parse JSON for document {}: {}", id, e);
                    e
                })?;
                // Ensure the id matches the filename
                doc.id = id.to_owned();

                self.verify_document(&doc, options).await?;

                trace!("Document {} retrieved successfully", id);
                Ok(Some(doc))
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("Document {} not found", id);
                Ok(None)
            },
            Err(e) => {
                error!("IO error reading document {}: {}", id, e);
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
    /// use sentinel_dbms::{Store, Collection};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
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

    /// Deletes a document from the collection (soft delete).
    ///
    /// Moves the JSON file corresponding to the given ID to a `.deleted/` subdirectory
    /// within the collection. This implements soft deletes, allowing for recovery
    /// of accidentally deleted documents. The `.deleted/` directory is created
    /// automatically if it doesn't exist.
    ///
    /// If the document doesn't exist, the operation succeeds silently (idempotent).
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
    /// use sentinel_dbms::{Store, Collection};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert a document
    /// collection.insert("user-123", json!({"name": "Alice"})).await?;
    ///
    /// // Soft delete the document
    /// collection.delete("user-123").await?;
    ///
    /// // Document is no longer accessible via get()
    /// let doc = collection.get("user-123").await?;
    /// assert!(doc.is_none());
    ///
    /// // But the file still exists in .deleted/
    /// // (can be recovered manually if needed)
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, id: &str) -> Result<()> {
        trace!("Deleting document with id: {}", id);
        validate_document_id(id)?;
        let source_path = self.path.join(format!("{}.json", id));
        let deleted_dir = self.path.join(".deleted");
        let dest_path = deleted_dir.join(format!("{}.json", id));

        // Check if source exists
        match tokio_fs::metadata(&source_path).await {
            Ok(_) => {
                debug!("Document {} exists, moving to .deleted", id);
                // Create .deleted directory if it doesn't exist
                tokio_fs::create_dir_all(&deleted_dir).await.map_err(|e| {
                    error!(
                        "Failed to create .deleted directory {:?}: {}",
                        deleted_dir, e
                    );
                    e
                })?;
                // Move file to .deleted/
                tokio_fs::rename(&source_path, &dest_path)
                    .await
                    .map_err(|e| {
                        error!("Failed to move document {} to .deleted: {}", id, e);
                        e
                    })?;
                debug!("Document {} soft deleted successfully", id);
                Ok(())
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!(
                    "Document {} not found, already deleted or never existed",
                    id
                );
                Ok(())
            },
            Err(e) => {
                error!("IO error checking document {} existence: {}", id, e);
                Err(SentinelError::Io {
                    source: e,
                })
            },
        }
    }

    /// Lists all document IDs in the collection.
    ///
    /// Returns a stream of document IDs from the collection directory.
    /// IDs are streamed as they are discovered, without guaranteed ordering.
    /// For sorted results, collect the stream and sort manually.
    ///
    /// # Returns
    ///
    /// Returns a stream of document IDs (filenames without the .json extension),
    /// or a `SentinelError` if the operation fails due to filesystem errors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::{Store, Collection};
    /// use serde_json::json;
    /// use futures::TryStreamExt;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert some documents
    /// collection.insert("user-123", json!({"name": "Alice"})).await?;
    /// collection.insert("user-456", json!({"name": "Bob"})).await?;
    ///
    /// // Stream all document IDs
    /// let ids: Vec<_> = collection.list().try_collect().await?;
    /// assert_eq!(ids.len(), 2);
    /// assert!(ids.contains(&"user-123".to_string()));
    /// assert!(ids.contains(&"user-456".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn list(&self) -> std::pin::Pin<Box<dyn Stream<Item = Result<String>> + Send>> {
        trace!("Streaming document IDs from collection: {}", self.name());
        stream_document_ids(self.path.clone())
    }

    /// Performs bulk insert operations on multiple documents.
    ///
    /// Inserts multiple documents into the collection in a single operation.
    /// If any document fails to insert, the operation stops and returns an error.
    /// Documents are inserted in the order provided.
    ///
    /// # Arguments
    ///
    /// * `documents` - A vector of (id, data) tuples to insert.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `SentinelError` if any operation fails.
    /// In case of failure, some documents may have been inserted before the error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::{Store, Collection};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Prepare bulk documents
    /// let documents = vec![
    ///     ("user-123", json!({"name": "Alice", "role": "admin"})),
    ///     ("user-456", json!({"name": "Bob", "role": "user"})),
    ///     ("user-789", json!({"name": "Charlie", "role": "user"})),
    /// ];
    ///
    /// // Bulk insert
    /// collection.bulk_insert(documents).await?;
    ///
    /// // Verify all documents were inserted
    /// assert!(collection.get("user-123").await?.is_some());
    /// assert!(collection.get("user-456").await?.is_some());
    /// assert!(collection.get("user-789").await?.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bulk_insert(&self, documents: Vec<(&str, Value)>) -> Result<()> {
        let count = documents.len();
        trace!(
            "Bulk inserting {} documents into collection {}",
            count,
            self.name()
        );
        for (id, data) in documents {
            self.insert(id, data).await?;
        }
        debug!("Bulk insert of {} documents completed successfully", count);
        Ok(())
    }

    /// Filters documents in the collection using a predicate function.
    ///
    /// This method performs streaming filtering by loading and checking documents
    /// one by one, keeping only matching documents in memory. This approach
    /// minimizes memory usage while maintaining good performance for most use cases.
    ///
    /// By default, this method verifies both hash and signature with strict mode.
    /// Use `filter_with_verification()` to customize verification behavior.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that takes a `&Document` and returns `true` if the document
    ///   should be included in the results.
    ///
    /// # Returns
    ///
    /// Returns a stream of documents that match the predicate.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::{Store, Collection};
    /// use serde_json::json;
    /// use futures::stream::StreamExt;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert some test data
    /// collection.insert("user-1", json!({"name": "Alice", "age": 25})).await?;
    /// collection.insert("user-2", json!({"name": "Bob", "age": 30})).await?;
    ///
    /// // Filter for users older than 26
    /// let mut adults = collection.filter(|doc| {
    ///     doc.data().get("age")
    ///         .and_then(|v| v.as_i64())
    ///         .map_or(false, |age| age > 26)
    /// });
    ///
    /// let mut count = 0;
    /// while let Some(doc) = adults.next().await {
    ///     let doc = doc?;
    ///     assert_eq!(doc.id(), "user-2");
    ///     count += 1;
    /// }
    /// assert_eq!(count, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn filter<F>(&self, predicate: F) -> std::pin::Pin<Box<dyn Stream<Item = Result<Document>> + Send>>
    where
        F: Fn(&Document) -> bool + Send + Sync + 'static,
    {
        self.filter_with_verification(predicate, &crate::VerificationOptions::default())
    }

    /// Filters documents in the collection using a predicate function with custom verification
    /// options.
    ///
    /// This method performs streaming filtering by loading and checking documents
    /// one by one, keeping only matching documents in memory. This approach
    /// minimizes memory usage while maintaining good performance for most use cases.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that takes a `&Document` and returns `true` if the document
    ///   should be included in the results.
    /// * `options` - Verification options controlling hash and signature verification.
    ///
    /// # Returns
    ///
    /// Returns a stream of documents that match the predicate.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::{Store, Collection, VerificationOptions};
    /// use serde_json::json;
    /// use futures::stream::StreamExt;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert some test data
    /// collection.insert("user-1", json!({"name": "Alice", "age": 25})).await?;
    /// collection.insert("user-2", json!({"name": "Bob", "age": 30})).await?;
    ///
    /// // Filter with warnings enabled
    /// let options = VerificationOptions::warn();
    /// let mut adults = collection.filter_with_verification(
    ///     |doc| {
    ///         doc.data().get("age")
    ///             .and_then(|v| v.as_i64())
    ///             .map_or(false, |age| age > 26)
    ///     },
    ///     &options
    /// );
    ///
    /// let mut count = 0;
    /// while let Some(doc) = adults.next().await {
    ///     let doc = doc?;
    ///     assert_eq!(doc.id(), "user-2");
    ///     count += 1;
    /// }
    /// assert_eq!(count, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn filter_with_verification<F>(
        &self,
        predicate: F,
        options: &crate::VerificationOptions,
    ) -> std::pin::Pin<Box<dyn Stream<Item = Result<Document>> + Send>>
    where
        F: Fn(&Document) -> bool + Send + Sync + 'static,
    {
        let collection_path = self.path.clone();
        let signing_key = self.signing_key.clone();
        let options = *options;

        Box::pin(stream! {
            trace!(
                "Streaming filter on collection (verification enabled: {})",
                options.verify_signature || options.verify_hash
            );
            let mut entries = match tokio_fs::read_dir(&collection_path).await {
                Ok(entries) => entries,
                Err(e) => {
                    yield Err(e.into());
                    return;
                }
            };

            loop {
                let entry = match entries.next_entry().await {
                    Ok(Some(entry)) => entry,
                    Ok(None) => break,
                    Err(e) => {
                        yield Err(e.into());
                        continue;
                    }
                };

                let path = entry.path();
                if !tokio_fs::metadata(&path).await.map(|m| m.is_dir()).unwrap_or(false)
                    && let Some(file_name) = path.file_name().and_then(|n| n.to_str())
                        && file_name.ends_with(".json") && !file_name.starts_with('.') {
                            let id = file_name.strip_suffix(".json").unwrap();
                            let file_path = collection_path.join(format!("{}.json", id));
                            match tokio_fs::read_to_string(&file_path).await {
                                Ok(content) => {
                                    match serde_json::from_str::<Document>(&content) {
                                        Ok(mut doc) => {
                                            doc.id = id.to_owned();

                                            let collection_ref = Collection {
                                                path: collection_path.clone(),
                                                signing_key: signing_key.clone(),
                                            };

                                            if let Err(e) = collection_ref.verify_document(&doc, &options).await {
                                                if matches!(e, SentinelError::HashVerificationFailed { .. } | SentinelError::SignatureVerificationFailed { .. }) {
                                                    if options.hash_verification_mode == crate::VerificationMode::Strict
                                                        || options.signature_verification_mode == crate::VerificationMode::Strict
                                                    {
                                                        yield Err(e);
                                                        continue;
                                                    }
                                                } else {
                                                    yield Err(e);
                                                    continue;
                                                }
                                            }

                                            if predicate(&doc) {
                                                yield Ok(doc);
                                            }
                                        }
                                        Err(e) => yield Err(e.into()),
                                    }
                                }
                                Err(e) => yield Err(e.into()),
                            }
                        }
            }
            debug!("Streaming filter completed");
        })
    }

    /// Streams all documents in the collection.
    ///
    /// This method performs streaming by loading documents one by one,
    /// minimizing memory usage.
    ///
    /// By default, this method verifies both hash and signature with strict mode.
    /// Use `all_with_verification()` to customize verification behavior.
    ///
    /// # Returns
    ///
    /// Returns a stream of all documents in the collection.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::{Collection, Store};
    /// use futures::stream::StreamExt;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Stream all documents
    /// let mut all_docs = collection.all();
    /// while let Some(doc) = all_docs.next().await {
    ///     let doc = doc?;
    ///     println!("Document: {}", doc.id());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn all(&self) -> std::pin::Pin<Box<dyn Stream<Item = Result<Document>> + Send>> {
        self.all_with_verification(&crate::VerificationOptions::default())
    }

    /// Streams all documents in the collection with custom verification options.
    ///
    /// This method performs streaming by loading documents one by one,
    /// minimizing memory usage.
    ///
    /// # Arguments
    ///
    /// * `options` - Verification options controlling hash and signature verification.
    ///
    /// # Returns
    ///
    /// Returns a stream of all documents in the collection.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::{Collection, Store, VerificationOptions};
    /// use futures::stream::StreamExt;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Stream all documents with warnings instead of errors
    /// let options = VerificationOptions::warn();
    /// let mut all_docs = collection.all_with_verification(&options);
    /// while let Some(doc) = all_docs.next().await {
    ///     let doc = doc?;
    ///     println!("Document: {}", doc.id());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn all_with_verification(
        &self,
        options: &crate::VerificationOptions,
    ) -> std::pin::Pin<Box<dyn Stream<Item = Result<Document>> + Send>> {
        let collection_path = self.path.clone();
        let signing_key = self.signing_key.clone();
        let options = *options;

        Box::pin(stream! {
            trace!(
                "Streaming all documents on collection (verification enabled: {})",
                options.verify_signature || options.verify_hash
            );
            let mut entries = match tokio_fs::read_dir(&collection_path).await {
                Ok(entries) => entries,
                Err(e) => {
                    yield Err(e.into());
                    return;
                }
            };

            loop {
                let entry = match entries.next_entry().await {
                    Ok(Some(entry)) => entry,
                    Ok(None) => break,
                    Err(e) => {
                        yield Err(e.into());
                        continue;
                    }
                };

                let path = entry.path();
                if !tokio_fs::metadata(&path).await.map(|m| m.is_dir()).unwrap_or(false)
                    && let Some(file_name) = path.file_name().and_then(|n| n.to_str())
                        && file_name.ends_with(".json") && !file_name.starts_with('.') {
                            let id = file_name.strip_suffix(".json").unwrap();
                            let file_path = collection_path.join(format!("{}.json", id));
                            match tokio_fs::read_to_string(&file_path).await {
                                Ok(content) => {
                                    match serde_json::from_str::<Document>(&content) {
                                        Ok(mut doc) => {
                                            doc.id = id.to_owned();

                                            let collection_ref = Collection {
                                                path: collection_path.clone(),
                                                signing_key: signing_key.clone(),
                                            };

                                            if let Err(e) = collection_ref.verify_document(&doc, &options).await {
                                                if matches!(e, SentinelError::HashVerificationFailed { .. } | SentinelError::SignatureVerificationFailed { .. }) {
                                                    if options.hash_verification_mode == crate::VerificationMode::Strict
                                                        || options.signature_verification_mode == crate::VerificationMode::Strict
                                                    {
                                                        yield Err(e);
                                                        continue;
                                                    }
                                                } else {
                                                    yield Err(e);
                                                    continue;
                                                }
                                            }

                                            yield Ok(doc);
                                        }
                                        Err(e) => yield Err(e.into()),
                                    }
                                }
                                Err(e) => yield Err(e.into()),
                            }
                        }
            }
            debug!("Streaming all completed");
        })
    }

    /// Executes a structured query against the collection.
    ///
    /// This method supports complex filtering, sorting, pagination, and field projection.
    /// For optimal performance and memory usage:
    /// - Queries without sorting use streaming processing with early limit application
    /// - Queries with sorting collect filtered documents in memory for sorting
    /// - Projection is applied only to final results to minimize memory usage
    ///
    /// By default, this method verifies both hash and signature with strict mode.
    /// Use `query_with_verification()` to customize verification behavior.
    ///
    /// # Arguments
    ///
    /// * `query` - The query to execute
    ///
    /// # Returns
    ///
    /// Returns a `QueryResult` containing the matching documents and metadata.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::{Store, Collection, QueryBuilder, Operator, SortOrder};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert test data
    /// collection.insert("user-1", json!({"name": "Alice", "age": 25, "city": "NYC"})).await?;
    /// collection.insert("user-2", json!({"name": "Bob", "age": 30, "city": "LA"})).await?;
    /// collection.insert("user-3", json!({"name": "Charlie", "age": 35, "city": "NYC"})).await?;
    ///
    /// // Query for users in NYC, sorted by age, limit 2
    /// let query = QueryBuilder::new()
    ///     .filter("city", Operator::Equals, json!("NYC"))
    ///     .sort("age", SortOrder::Ascending)
    ///     .limit(2)
    ///     .projection(vec!["name", "age"])
    ///     .build();
    ///
    /// let result = collection.query(query).await?;
    /// let documents: Vec<_> = futures::TryStreamExt::try_collect(result.documents).await?;
    /// assert_eq!(documents.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query(&self, query: crate::Query) -> Result<crate::QueryResult> {
        self.query_with_verification(query, &crate::VerificationOptions::default())
            .await
    }

    /// Executes a structured query against the collection with custom verification options.
    ///
    /// This method supports complex filtering, sorting, pagination, and field projection.
    /// For optimal performance and memory usage:
    /// - Queries without sorting use streaming processing with early limit application
    /// - Queries with sorting collect filtered documents in memory for sorting
    /// - Projection is applied only to final results to minimize memory usage
    ///
    /// # Arguments
    ///
    /// * `query` - The query to execute
    /// * `options` - Verification options controlling hash and signature verification.
    ///
    /// # Returns
    ///
    /// Returns a `QueryResult` containing the matching documents and metadata.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::{Store, Collection, QueryBuilder, Operator, SortOrder, VerificationOptions, VerificationMode};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert test data
    /// collection.insert("user-1", json!({"name": "Alice", "age": 25, "city": "NYC"})).await?;
    /// collection.insert("user-2", json!({"name": "Bob", "age": 30, "city": "LA"})).await?;
    /// collection.insert("user-3", json!({"name": "Charlie", "age": 35, "city": "NYC"})).await?;
    ///
    /// // Query with warning mode
    /// let options = VerificationOptions::warn();
    /// let query = QueryBuilder::new()
    ///     .filter("city", Operator::Equals, json!("NYC"))
    ///     .sort("age", SortOrder::Ascending)
    ///     .limit(2)
    ///     .projection(vec!["name", "age"])
    ///     .build();
    ///
    /// let result = collection.query_with_verification(query, &options).await?;
    /// let documents: Vec<_> = futures::TryStreamExt::try_collect(result.documents).await?;
    /// assert_eq!(documents.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_with_verification(
        &self,
        query: crate::Query,
        options: &crate::VerificationOptions,
    ) -> Result<crate::QueryResult> {
        use std::time::Instant;
        let start_time = Instant::now();

        trace!(
            "Executing query on collection: {} (verification enabled: {})",
            self.name(),
            options.verify_signature || options.verify_hash
        );

        // Get all document IDs - but for full streaming, we should avoid this
        // However, for sorted queries, we need to know all IDs to collect
        // For non-sorted, we can stream without knowing all IDs
        let documents_stream = if query.sort.is_some() {
            // For sorted queries, we need to collect all matching documents
            let all_ids: Vec<String> = self.list().try_collect().await?;
            let docs = self
                .execute_sorted_query_with_verification(&all_ids, &query, options)
                .await?;
            let stream = tokio_stream::iter(docs.into_iter().map(Ok));
            Box::pin(stream) as std::pin::Pin<Box<dyn Stream<Item = Result<Document>> + Send>>
        }
        else {
            // For non-sorted queries, use streaming
            self.execute_streaming_query_with_verification(&query, options)
                .await?
        };

        let execution_time = start_time.elapsed();
        debug!("Query completed in {:?}", execution_time);

        Ok(crate::QueryResult {
            documents: documents_stream,
            total_count: None, // For streaming, we don't know the total count upfront
            execution_time,
        })
    }

    /// Executes a query that requires sorting by collecting all matching documents first.
    async fn execute_sorted_query(&self, all_ids: &[String], query: &crate::Query) -> Result<Vec<Document>> {
        self.execute_sorted_query_with_verification(all_ids, query, &crate::VerificationOptions::default())
            .await
    }

    /// Executes a query that requires sorting by collecting all matching documents first with
    /// verification.
    async fn execute_sorted_query_with_verification(
        &self,
        all_ids: &[String],
        query: &crate::Query,
        options: &crate::VerificationOptions,
    ) -> Result<Vec<Document>> {
        // For sorted queries, we need to collect all matching documents to sort them
        // But we can optimize by only keeping document IDs and sort values during filtering
        let mut matching_docs = Vec::new();

        // Precompute filter references to avoid allocating a new Vec for each document
        let filter_refs: Vec<_> = query.filters.iter().collect();

        for id in all_ids {
            if let Some(doc) = self.get_with_verification(id, options).await? &&
                matches_filters(&doc, &filter_refs)
            {
                matching_docs.push(doc);
            }
        }

        if let Some(ref inner) = query.sort {
            let field = &inner.0;
            let order = &inner.1;
            matching_docs.sort_by(|a, b| {
                let a_val = a.data().get(field.as_str());
                let b_val = b.data().get(field.as_str());
                if *order == crate::SortOrder::Ascending {
                    self.compare_values(a_val, b_val)
                }
                else {
                    self.compare_values(b_val, a_val)
                }
            });
        }

        // Apply offset and limit
        let offset = query.offset.unwrap_or(0);
        let start_idx = offset.min(matching_docs.len());
        let end_idx = query.limit.map_or(matching_docs.len(), |limit| {
            start_idx.saturating_add(limit).min(matching_docs.len())
        });

        // Apply projection to the final results
        let mut final_docs = Vec::new();
        for doc in matching_docs
            .into_iter()
            .skip(start_idx)
            .take(end_idx.saturating_sub(start_idx))
        {
            let projected_doc = if let Some(ref fields) = query.projection {
                self.project_document(&doc, fields).await?
            }
            else {
                doc
            };
            final_docs.push(projected_doc);
        }

        Ok(final_docs)
    }

    /// Executes a query without sorting, allowing streaming with early limit application.
    async fn execute_streaming_query(
        &self,
        query: &crate::Query,
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<Document>> + Send>>> {
        self.execute_streaming_query_with_verification(query, &crate::VerificationOptions::default())
            .await
    }

    /// Executes a query without sorting, allowing streaming with early limit application and
    /// verification.
    async fn execute_streaming_query_with_verification(
        &self,
        query: &crate::Query,
        options: &crate::VerificationOptions,
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<Document>> + Send>>> {
        let collection_path = self.path.clone();
        let signing_key = self.signing_key.clone();
        let filters = query.filters.clone();
        let projection_fields = query.projection.clone();
        let limit = query.limit.unwrap_or(usize::MAX);
        let offset = query.offset.unwrap_or(0);
        let options = *options;

        Ok(Box::pin(stream! {
            let mut id_stream = stream_document_ids(collection_path.clone());
            let mut yielded = 0;
            let mut skipped = 0;

            // Precompute filter references to avoid allocating a new Vec for each document
            let filter_refs: Vec<_> = filters.iter().collect();

            while let Some(id_result) = id_stream.next().await {
                let id = match id_result {
                    Ok(id) => id,
                    Err(e) => {
                        yield Err(e);
                        continue;
                    }
                };

                // Load document
                let file_path = collection_path.join(format!("{}.json", id));
                let content = match tokio_fs::read_to_string(&file_path).await {
                    Ok(content) => content,
                    Err(e) => {
                        yield Err(e.into());
                        continue;
                    }
                };

                let doc = match serde_json::from_str::<Document>(&content) {
                    Ok(doc) => {
                        // Create a new document with the correct ID
                        let mut doc_with_id = doc;
                        doc_with_id.id = id.clone();

                        let collection_ref = Collection {
                            path: collection_path.clone(),
                            signing_key: signing_key.clone(),
                        };

                        if let Err(e) = collection_ref.verify_document(&doc_with_id, &options).await {
                            if matches!(e, SentinelError::HashVerificationFailed { .. } | SentinelError::SignatureVerificationFailed { .. }) {
                                if options.hash_verification_mode == crate::VerificationMode::Strict
                                    || options.signature_verification_mode == crate::VerificationMode::Strict
                                {
                                    yield Err(e);
                                    continue;
                                }
                            } else {
                                yield Err(e);
                                continue;
                            }
                        }

                        doc_with_id
                    }
                    Err(e) => {
                        yield Err(e.into());
                        continue;
                    }
                };

                if matches_filters(&doc, &filter_refs) {
                    if skipped < offset {
                        skipped = skipped.saturating_add(1);
                        continue;
                    }
                    if yielded >= limit {
                        break;
                    }
                    let final_doc = if let Some(ref fields) = projection_fields {
                        project_document(&doc, fields).await?
                    } else {
                        doc
                    };
                    yield Ok(final_doc);
                    yielded = yielded.saturating_add(1);
                }
            }
        }))
    }

    /// Compares two values for sorting purposes.
    fn compare_values(&self, a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering { compare_values(a, b) }

    /// Projects a document to include only specified fields.
    async fn project_document(&self, doc: &Document, fields: &[String]) -> Result<Document> {
        project_document(doc, fields).await
    }

    /// Verifies document hash according to the specified verification mode.
    ///
    /// # Arguments
    ///
    /// * `doc` - The document to verify
    /// * `mode` - The verification mode (Strict, Warn, or Silent)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if verification passes or is in Warn/Silent mode,
    /// or `Err(SentinelError::HashVerificationFailed)` if verification fails in Strict mode.
    async fn verify_hash(&self, doc: &Document, mode: crate::VerificationMode) -> Result<()> {
        if mode == crate::VerificationMode::Silent {
            return Ok(());
        }

        trace!("Verifying hash for document: {}", doc.id());
        let computed_hash = sentinel_crypto::hash_data(doc.data()).await?;

        if computed_hash != doc.hash() {
            let reason = format!(
                "Expected hash: {}, Computed hash: {}",
                doc.hash(),
                computed_hash
            );

            match mode {
                crate::VerificationMode::Strict => {
                    error!("Document {} hash verification failed: {}", doc.id(), reason);
                    return Err(SentinelError::HashVerificationFailed {
                        id: doc.id().to_string(),
                        reason,
                    });
                },
                crate::VerificationMode::Warn => {
                    warn!("Document {} hash verification failed: {}", doc.id(), reason);
                },
                crate::VerificationMode::Silent => {},
            }
        }
        else {
            trace!("Document {} hash verified successfully", doc.id());
        }

        Ok(())
    }

    /// Verifies document signature according to the specified verification mode.
    ///
    /// # Arguments
    ///
    /// * `doc` - The document to verify
    /// * `mode` - The verification mode (Strict, Warn, or Silent)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if verification passes or is in Warn/Silent mode,
    /// or `Err(SentinelError::SignatureVerificationFailed)` if verification fails in Strict mode.
    async fn verify_signature(&self, doc: &Document, mode: crate::VerificationMode) -> Result<()> {
        if mode == crate::VerificationMode::Silent {
            return Ok(());
        }

        trace!("Verifying signature for document: {}", doc.id());

        if doc.signature().is_empty() {
            let reason = "Document has no signature".to_string();

            match mode {
                crate::VerificationMode::Strict => {
                    error!(
                        "Document {} signature verification failed: {}",
                        doc.id(),
                        reason
                    );
                    return Err(SentinelError::SignatureVerificationFailed {
                        id: doc.id().to_string(),
                        reason,
                    });
                },
                crate::VerificationMode::Warn => {
                    warn!(
                        "Document {} signature verification failed: {}",
                        doc.id(),
                        reason
                    );
                },
                crate::VerificationMode::Silent => {},
            }
            return Ok(());
        }

        if let Some(ref signing_key) = self.signing_key {
            let public_key = signing_key.verifying_key();
            let is_valid = sentinel_crypto::verify_signature(doc.hash(), doc.signature(), &public_key).await?;

            if !is_valid {
                let reason = "Signature verification using public key failed".to_string();

                match mode {
                    crate::VerificationMode::Strict => {
                        error!(
                            "Document {} signature verification failed: {}",
                            doc.id(),
                            reason
                        );
                        return Err(SentinelError::SignatureVerificationFailed {
                            id: doc.id().to_string(),
                            reason,
                        });
                    },
                    crate::VerificationMode::Warn => {
                        warn!(
                            "Document {} signature verification failed: {}",
                            doc.id(),
                            reason
                        );
                    },
                    crate::VerificationMode::Silent => {},
                }
            }
            else {
                trace!("Document {} signature verified successfully", doc.id());
            }
        }
        else {
            trace!("No signing key available for verification, skipping signature check");
        }

        Ok(())
    }

    /// Verifies both hash and signature of a document according to the specified options.
    ///
    /// # Arguments
    ///
    /// * `doc` - The document to verify
    /// * `options` - The verification options
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if verifications pass or are in Warn/Silent mode,
    /// or an error if verification fails in Strict mode.
    async fn verify_document(&self, doc: &Document, options: &crate::VerificationOptions) -> Result<()> {
        if options.verify_hash {
            self.verify_hash(doc, options.hash_verification_mode)
                .await?;
        }

        if options.verify_signature {
            self.verify_signature(doc, options.signature_verification_mode)
                .await?;
        }

        Ok(())
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
pub fn validate_document_id(id: &str) -> Result<()> {
    trace!("Validating document id: {}", id);
    // Check if id is empty
    if id.is_empty() {
        warn!("Document id is empty");
        return Err(SentinelError::InvalidDocumentId {
            id: id.to_owned(),
        });
    }

    // Check for valid characters
    if !is_valid_document_id_chars(id) {
        warn!("Document id contains invalid characters: {}", id);
        return Err(SentinelError::InvalidDocumentId {
            id: id.to_owned(),
        });
    }

    // Check for Windows reserved names
    if is_reserved_name(id) {
        warn!("Document id is a reserved name: {}", id);
        return Err(SentinelError::InvalidDocumentId {
            id: id.to_owned(),
        });
    }

    trace!("Document id '{}' is valid", id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::tempdir;
    use tracing_subscriber;

    use super::*;
    use crate::Store;

    /// Helper function to set up a temporary collection for testing
    async fn setup_collection() -> (Collection, tempfile::TempDir) {
        // Initialize tracing for tests to ensure debug! macros are executed
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .try_init();

        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();
        (collection, temp_dir)
    }

    /// Helper function to set up a temporary collection with signing key for testing
    async fn setup_collection_with_signing_key() -> (Collection, tempfile::TempDir) {
        // Initialize tracing for tests to ensure debug! macros are executed
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .try_init();

        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();
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
    async fn test_insert_with_signing_key() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Alice", "signed": true });
        collection.insert("signed_doc", doc.clone()).await.unwrap();

        let retrieved = collection.get("signed_doc").await.unwrap().unwrap();
        assert_eq!(*retrieved.data(), doc);
        // Check that signature is not empty
        assert!(!retrieved.signature().is_empty());
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

        // Check that file was moved to .deleted/
        let deleted_path = collection.path.join(".deleted").join("user-123.json");
        assert!(tokio_fs::try_exists(&deleted_path).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let (collection, _temp_dir) = setup_collection().await;

        // Should not error
        collection.delete("nonexistent").await.unwrap();
    }

    #[tokio::test]
    async fn test_list_empty_collection() {
        let (collection, _temp_dir) = setup_collection().await;

        let ids: Vec<String> = collection.list().try_collect().await.unwrap();
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn test_list_with_documents() {
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("user-123", json!({"name": "Alice"}))
            .await
            .unwrap();
        collection
            .insert("user-456", json!({"name": "Bob"}))
            .await
            .unwrap();
        collection
            .insert("user-789", json!({"name": "Charlie"}))
            .await
            .unwrap();

        let mut ids: Vec<String> = collection.list().try_collect().await.unwrap();
        ids.sort(); // Sort for consistent ordering in test
        assert_eq!(ids.len(), 3);
        assert_eq!(ids, vec!["user-123", "user-456", "user-789"]);
    }

    #[tokio::test]
    async fn test_list_skips_deleted_documents() {
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("user-123", json!({"name": "Alice"}))
            .await
            .unwrap();
        collection
            .insert("user-456", json!({"name": "Bob"}))
            .await
            .unwrap();
        collection.delete("user-456").await.unwrap();

        let ids: Vec<String> = collection.list().try_collect().await.unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids, vec!["user-123"]);
    }

    #[tokio::test]
    async fn test_bulk_insert() {
        let (collection, _temp_dir) = setup_collection().await;

        let documents = vec![
            ("user-123", json!({"name": "Alice", "role": "admin"})),
            ("user-456", json!({"name": "Bob", "role": "user"})),
            ("user-789", json!({"name": "Charlie", "role": "user"})),
        ];

        collection.bulk_insert(documents).await.unwrap();

        let ids: Vec<String> = collection.list().try_collect().await.unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"user-123".to_string()));
        assert!(ids.contains(&"user-456".to_string()));
        assert!(ids.contains(&"user-789".to_string()));

        // Verify data
        let alice = collection.get("user-123").await.unwrap().unwrap();
        assert_eq!(alice.data()["name"], "Alice");
        assert_eq!(alice.data()["role"], "admin");
    }

    #[tokio::test]
    async fn test_bulk_insert_empty() {
        let (collection, _temp_dir) = setup_collection().await;

        collection.bulk_insert(vec![]).await.unwrap();

        let ids: Vec<String> = collection.list().try_collect().await.unwrap();
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn test_bulk_insert_with_invalid_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let documents = vec![
            ("user-123", json!({"name": "Alice"})),
            ("user!invalid", json!({"name": "Bob"})),
        ];

        let result = collection.bulk_insert(documents).await;
        assert!(result.is_err());

        // First document should have been inserted before error
        let ids: Vec<String> = collection.list().try_collect().await.unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "user-123");
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
        assert!(validate_document_id("file🚀name").is_err()); // emoji
        assert!(validate_document_id("fileéname").is_err()); // accented
        assert!(validate_document_id("file.name").is_err()); // dot
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
    async fn test_get_corrupted_json() {
        let (collection, _temp_dir) = setup_collection().await;

        // Manually create a file with invalid JSON
        let file_path = collection.path.join("corrupted.json");
        tokio_fs::write(&file_path, "{ invalid json }")
            .await
            .unwrap();

        let result = collection.get("corrupted").await;
        assert!(result.is_err());
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

    #[tokio::test]
    async fn test_get_with_verification_strict_mode_valid_signature() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Alice", "signed": true });
        collection
            .insert("valid_signed", doc.clone())
            .await
            .unwrap();

        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Strict,
            hash_verification_mode:      crate::VerificationMode::Strict,
        };

        let result = collection
            .get_with_verification("valid_signed", &options)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_get_with_verification_strict_mode_invalid_signature() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Alice", "data": "original" });
        collection.insert("original_doc", doc).await.unwrap();

        let file_path = collection.path.join("original_doc.json");
        let mut content = tokio_fs::read_to_string(&file_path).await.unwrap();
        let mut json_value: serde_json::Value = serde_json::from_str(&content).unwrap();
        json_value["data"] = json!("tampered");
        let tampered_content = serde_json::to_string(&json_value).unwrap();
        tokio_fs::write(&file_path, tampered_content).await.unwrap();

        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Strict,
            hash_verification_mode:      crate::VerificationMode::Strict,
        };

        let result = collection
            .get_with_verification("original_doc", &options)
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::SentinelError::HashVerificationFailed {
                ..
            } => {},
            crate::SentinelError::SignatureVerificationFailed {
                ..
            } => {},
            _ => panic!("Expected hash or signature verification failure"),
        }
    }

    #[tokio::test]
    async fn test_get_with_verification_warn_mode_invalid_signature() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Bob" });
        collection.insert("warn_test_doc", doc).await.unwrap();

        let file_path = collection.path.join("warn_test_doc.json");
        let mut content = tokio_fs::read_to_string(&file_path).await.unwrap();
        let mut json_value: serde_json::Value = serde_json::from_str(&content).unwrap();
        json_value["name"] = json!("tampered");
        let tampered_content = serde_json::to_string(&json_value).unwrap();
        tokio_fs::write(&file_path, tampered_content).await.unwrap();

        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Warn,
            hash_verification_mode:      crate::VerificationMode::Warn,
        };

        let result = collection
            .get_with_verification("warn_test_doc", &options)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_get_with_verification_silent_mode() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Charlie" });
        collection.insert("silent_test_doc", doc).await.unwrap();

        let file_path = collection.path.join("silent_test_doc.json");
        let mut content = tokio_fs::read_to_string(&file_path).await.unwrap();
        let mut json_value: serde_json::Value = serde_json::from_str(&content).unwrap();
        json_value["name"] = json!("silently_tampered");
        let tampered_content = serde_json::to_string(&json_value).unwrap();
        tokio_fs::write(&file_path, tampered_content).await.unwrap();

        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Silent,
            hash_verification_mode:      crate::VerificationMode::Silent,
        };

        let result = collection
            .get_with_verification("silent_test_doc", &options)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_get_with_verification_disabled() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Dave" });
        collection.insert("disabled_test_doc", doc).await.unwrap();

        let file_path = collection.path.join("disabled_test_doc.json");
        let mut content = tokio_fs::read_to_string(&file_path).await.unwrap();
        let mut json_value: serde_json::Value = serde_json::from_str(&content).unwrap();
        json_value["name"] = json!("tampered");
        let tampered_content = serde_json::to_string(&json_value).unwrap();
        tokio_fs::write(&file_path, tampered_content).await.unwrap();

        let options = crate::VerificationOptions::disabled();

        let result = collection
            .get_with_verification("disabled_test_doc", &options)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_all_with_verification_strict_mode() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        collection
            .insert("doc1", json!({ "name": "Alice" }))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({ "name": "Bob" }))
            .await
            .unwrap();

        let options = crate::VerificationOptions::strict();
        let mut stream = collection.all_with_verification(&options);
        let mut count = 0;

        use futures::StreamExt;
        while let Some(result) = stream.next().await {
            assert!(result.is_ok());
            count += 1;
        }

        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_filter_with_verification_strict_mode() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        collection
            .insert("doc1", json!({ "name": "Alice", "age": 25 }))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({ "name": "Bob", "age": 30 }))
            .await
            .unwrap();

        let options = crate::VerificationOptions::strict();
        let mut stream = collection.filter_with_verification(
            |doc| {
                doc.data()
                    .get("age")
                    .and_then(|v| v.as_i64())
                    .map_or(false, |age| age >= 25)
            },
            &options,
        );

        use futures::StreamExt;
        let mut count = 0;
        while let Some(result) = stream.next().await {
            assert!(result.is_ok());
            count += 1;
        }

        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_query_with_verification_strict_mode() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        collection
            .insert("doc1", json!({ "name": "Alice", "age": 25 }))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({ "name": "Bob", "age": 30 }))
            .await
            .unwrap();

        let options = crate::VerificationOptions::strict();
        let query = crate::QueryBuilder::new()
            .filter("age", crate::Operator::GreaterOrEqual, json!(25))
            .build();

        let result = collection.query_with_verification(query, &options).await;
        assert!(result.is_ok());

        use futures::StreamExt;
        let mut stream = result.unwrap().documents;
        let mut count = 0;
        while let Some(doc_result) = stream.next().await {
            assert!(doc_result.is_ok());
            count += 1;
        }

        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_filter_empty_collection() {
        let (collection, _temp_dir) = setup_collection().await;

        let stream = collection.filter(|_| true);
        let results: Result<Vec<_>> = futures::TryStreamExt::try_collect(stream).await;
        assert!(results.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_filter_all_documents() {
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("user-1", json!({"name": "Alice", "age": 25}))
            .await
            .unwrap();
        collection
            .insert("user-2", json!({"name": "Bob", "age": 30}))
            .await
            .unwrap();

        let stream = collection.filter(|_| true);
        let results: Result<Vec<_>> = futures::TryStreamExt::try_collect(stream).await;
        assert_eq!(results.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_filter_by_predicate() {
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("user-1", json!({"name": "Alice", "age": 25}))
            .await
            .unwrap();
        collection
            .insert("user-2", json!({"name": "Bob", "age": 30}))
            .await
            .unwrap();
        collection
            .insert("user-3", json!({"name": "Charlie", "age": 35}))
            .await
            .unwrap();

        let stream = collection.filter(|doc| {
            doc.data()
                .get("age")
                .and_then(|v| v.as_i64())
                .map_or(false, |age| age > 26)
        });
        let results: Result<Vec<_>> = futures::TryStreamExt::try_collect(stream).await;
        let results = results.unwrap();

        assert_eq!(results.len(), 2);
        let names: Vec<&str> = results
            .iter()
            .map(|d| d.data()["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"Bob"));
        assert!(names.contains(&"Charlie"));
    }

    #[tokio::test]
    async fn test_query_empty_collection() {
        let (collection, _temp_dir) = setup_collection().await;

        let query = crate::QueryBuilder::new().build();
        let result = collection.query(query).await.unwrap();
        let documents: Result<Vec<_>> = futures::TryStreamExt::try_collect(result.documents).await;
        assert!(documents.unwrap().is_empty());
        assert_eq!(result.total_count, None);
    }

    #[tokio::test]
    async fn test_query_with_filters() {
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("user-1", json!({"name": "Alice", "age": 25, "city": "NYC"}))
            .await
            .unwrap();
        collection
            .insert("user-2", json!({"name": "Bob", "age": 30, "city": "LA"}))
            .await
            .unwrap();
        collection
            .insert(
                "user-3",
                json!({"name": "Charlie", "age": 35, "city": "NYC"}),
            )
            .await
            .unwrap();

        let query = crate::QueryBuilder::new()
            .filter("city", crate::Operator::Equals, json!("NYC"))
            .build();

        let result = collection.query(query).await.unwrap();
        let documents: Result<Vec<_>> = futures::TryStreamExt::try_collect(result.documents).await;
        let documents = documents.unwrap();
        assert_eq!(documents.len(), 2);
        assert_eq!(result.total_count, None);
    }

    #[tokio::test]
    async fn test_query_with_sorting() {
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("user-1", json!({"name": "Alice", "age": 25}))
            .await
            .unwrap();
        collection
            .insert("user-2", json!({"name": "Bob", "age": 30}))
            .await
            .unwrap();
        collection
            .insert("user-3", json!({"name": "Charlie", "age": 20}))
            .await
            .unwrap();

        let query = crate::QueryBuilder::new()
            .sort("age", crate::SortOrder::Ascending)
            .build();

        let result = collection.query(query).await.unwrap();
        let documents: Result<Vec<_>> = futures::TryStreamExt::try_collect(result.documents).await;
        let documents = documents.unwrap();
        assert_eq!(documents.len(), 3);
        assert_eq!(documents[0].data()["name"], "Charlie");
        assert_eq!(documents[1].data()["name"], "Alice");
        assert_eq!(documents[2].data()["name"], "Bob");
    }

    #[tokio::test]
    async fn test_query_with_limit_and_offset() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 1 ..= 5 {
            collection
                .insert(&format!("user-{}", i), json!({"id": i}))
                .await
                .unwrap();
        }

        let query = crate::QueryBuilder::new().limit(2).offset(1).build();

        let result = collection.query(query).await.unwrap();
        let documents: Result<Vec<_>> = futures::TryStreamExt::try_collect(result.documents).await;
        let documents = documents.unwrap();
        assert_eq!(documents.len(), 2);
        assert_eq!(result.total_count, None);
    }

    #[tokio::test]
    async fn test_query_with_projection() {
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("user-1", json!({"name": "Alice", "age": 25, "city": "NYC"}))
            .await
            .unwrap();

        let query = crate::QueryBuilder::new()
            .projection(vec!["name", "age"])
            .build();

        let result = collection.query(query).await.unwrap();
        let documents: Result<Vec<_>> = futures::TryStreamExt::try_collect(result.documents).await;
        let documents = documents.unwrap();
        assert_eq!(documents.len(), 1);
        let doc = &documents[0];
        if let Value::Object(map) = doc.data() {
            assert!(map.contains_key("name"));
            assert!(map.contains_key("age"));
            assert!(!map.contains_key("city"));
        }
        else {
            panic!("Document data should be an object");
        }
    }

    #[tokio::test]
    async fn test_query_string_filters() {
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert(
                "user-1",
                json!({"name": "Alice", "email": "alice@example.com"}),
            )
            .await
            .unwrap();
        collection
            .insert("user-2", json!({"name": "Bob", "email": "bob@test.com"}))
            .await
            .unwrap();

        let query = crate::QueryBuilder::new()
            .filter("email", crate::Operator::Contains, json!("example"))
            .build();

        let result = collection.query(query).await.unwrap();
        let documents: Result<Vec<_>> = futures::TryStreamExt::try_collect(result.documents).await;
        let documents = documents.unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].data()["name"], "Alice");
    }

    #[tokio::test]
    async fn test_filter_with_corrupted_json() {
        let (collection, _temp_dir) = setup_collection().await;

        // Insert a valid document
        collection
            .insert("valid", json!({"name": "Alice"}))
            .await
            .unwrap();

        // Manually create a corrupted JSON file
        let corrupted_path = collection.path.join("corrupted.json");
        tokio_fs::write(&corrupted_path, "{ invalid json }")
            .await
            .unwrap();

        let stream = collection.filter(|_| true);
        let results: Vec<Result<Document>> = stream.collect().await;

        // Should have one valid document and one error
        assert_eq!(results.len(), 2);
        let ok_count = results.iter().filter(|r| r.is_ok()).count();
        let err_count = results.iter().filter(|r| r.is_err()).count();
        assert_eq!(ok_count, 1);
        assert_eq!(err_count, 1);
    }

    #[tokio::test]
    async fn test_filter_contains_comprehensive() {
        let (collection, _temp_dir) = setup_collection().await;

        // Test Contains on string field (should match)
        collection
            .insert("doc1", json!({"text": "Hello World"}))
            .await
            .unwrap();

        // Test Contains on array field with strings (should match)
        collection
            .insert("doc2", json!({"tags": ["rust", "programming", "async"]}))
            .await
            .unwrap();

        // Test Contains on array field with mixed types (should not match non-strings)
        collection
            .insert("doc3", json!({"mixed": ["string", 123, true]}))
            .await
            .unwrap();

        // Test Contains on non-string, non-array field (should not match)
        collection
            .insert("doc4", json!({"number": 42}))
            .await
            .unwrap();

        // Query for string contains
        let result = collection
            .query(
                crate::QueryBuilder::new()
                    .filter("text", crate::Operator::Contains, json!("World"))
                    .build(),
            )
            .await
            .unwrap();
        let documents: Result<Vec<_>> = futures::TryStreamExt::try_collect(result.documents).await;
        let documents = documents.unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].id(), "doc1");

        // Query for array contains (string in array)
        let result = collection
            .query(
                crate::QueryBuilder::new()
                    .filter("tags", crate::Operator::Contains, json!("rust"))
                    .build(),
            )
            .await
            .unwrap();
        let documents: Result<Vec<_>> = futures::TryStreamExt::try_collect(result.documents).await;
        let documents = documents.unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].id(), "doc2");

        // Query for array contains (non-existent string)
        let result = collection
            .query(
                crate::QueryBuilder::new()
                    .filter("tags", crate::Operator::Contains, json!("python"))
                    .build(),
            )
            .await
            .unwrap();
        let documents: Result<Vec<_>> = futures::TryStreamExt::try_collect(result.documents).await;
        let documents = documents.unwrap();
        assert_eq!(documents.len(), 0);

        // Query for mixed array contains (should not match numbers/bools)
        let result = collection
            .query(
                crate::QueryBuilder::new()
                    .filter("mixed", crate::Operator::Contains, json!("string"))
                    .build(),
            )
            .await
            .unwrap();
        let documents: Result<Vec<_>> = futures::TryStreamExt::try_collect(result.documents).await;
        let documents = documents.unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].id(), "doc3");

        // Query for number field contains (should not match)
        let result = collection
            .query(
                crate::QueryBuilder::new()
                    .filter("number", crate::Operator::Contains, json!("42"))
                    .build(),
            )
            .await
            .unwrap();
        let documents: Result<Vec<_>> = futures::TryStreamExt::try_collect(result.documents).await;
        let documents = documents.unwrap();
        assert_eq!(documents.len(), 0);
    }

    #[tokio::test]
    async fn test_all_documents() {
        let (collection, _temp) = setup_collection().await;

        // Insert some documents
        collection
            .insert("doc1", json!({"name": "Alice"}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"name": "Bob"}))
            .await
            .unwrap();

        // Stream all documents
        let all_docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert_eq!(all_docs.len(), 2);

        let ids: std::collections::HashSet<_> = all_docs.iter().map(|d| d.id()).collect();
        assert!(ids.contains("doc1"));
        assert!(ids.contains("doc2"));
    }

    #[tokio::test]
    async fn test_get_with_verification_strict_mode_valid_signature() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Alice", "signed": true });
        collection
            .insert("valid_signed", doc.clone())
            .await
            .unwrap();

        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Strict,
            hash_verification_mode:      crate::VerificationMode::Strict,
        };

        let result = collection
            .get_with_verification("valid_signed", &options)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_get_with_verification_strict_mode_invalid_signature() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Alice", "data": "original" });
        collection.insert("original_doc", doc).await.unwrap();

        let file_path = collection.path.join("original_doc.json");
        let mut content = tokio_fs::read_to_string(&file_path).await.unwrap();
        let mut json_value: serde_json::Value = serde_json::from_str(&content).unwrap();
        json_value["data"] = json!("tampered");
        let tampered_content = serde_json::to_string(&json_value).unwrap();
        tokio_fs::write(&file_path, tampered_content).await.unwrap();

        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Strict,
            hash_verification_mode:      crate::VerificationMode::Strict,
        };

        let result = collection
            .get_with_verification("original_doc", &options)
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::SentinelError::HashVerificationFailed {
                ..
            } => {},
            crate::SentinelError::SignatureVerificationFailed {
                ..
            } => {},
            _ => panic!("Expected hash or signature verification failure"),
        }
    }

    #[tokio::test]
    async fn test_get_with_verification_warn_mode_invalid_signature() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Bob" });
        collection.insert("warn_test_doc", doc).await.unwrap();

        let file_path = collection.path.join("warn_test_doc.json");
        let mut content = tokio_fs::read_to_string(&file_path).await.unwrap();
        let mut json_value: serde_json::Value = serde_json::from_str(&content).unwrap();
        json_value["name"] = json!("tampered");
        let tampered_content = serde_json::to_string(&json_value).unwrap();
        tokio_fs::write(&file_path, tampered_content).await.unwrap();

        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Warn,
            hash_verification_mode:      crate::VerificationMode::Warn,
        };

        let result = collection
            .get_with_verification("warn_test_doc", &options)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_get_with_verification_silent_mode() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Charlie" });
        collection.insert("silent_test_doc", doc).await.unwrap();

        let file_path = collection.path.join("silent_test_doc.json");
        let mut content = tokio_fs::read_to_string(&file_path).await.unwrap();
        let mut json_value: serde_json::Value = serde_json::from_str(&content).unwrap();
        json_value["name"] = json!("silently_tampered");
        let tampered_content = serde_json::to_string(&json_value).unwrap();
        tokio_fs::write(&file_path, tampered_content).await.unwrap();

        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Silent,
            hash_verification_mode:      crate::VerificationMode::Silent,
        };

        let result = collection
            .get_with_verification("silent_test_doc", &options)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_get_with_verification_disabled() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Dave" });
        collection.insert("disabled_test_doc", doc).await.unwrap();

        let file_path = collection.path.join("disabled_test_doc.json");
        let mut content = tokio_fs::read_to_string(&file_path).await.unwrap();
        let mut json_value: serde_json::Value = serde_json::from_str(&content).unwrap();
        json_value["name"] = json!("tampered");
        let tampered_content = serde_json::to_string(&json_value).unwrap();
        tokio_fs::write(&file_path, tampered_content).await.unwrap();

        let options = crate::VerificationOptions::disabled();

        let result = collection
            .get_with_verification("disabled_test_doc", &options)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_all_with_verification_strict_mode() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        collection
            .insert("doc1", json!({ "name": "Alice" }))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({ "name": "Bob" }))
            .await
            .unwrap();

        let options = crate::VerificationOptions::strict();
        let mut stream = collection.all_with_verification(&options);
        let mut count = 0;

        use futures::StreamExt;
        while let Some(result) = stream.next().await {
            assert!(result.is_ok());
            count = count.saturating_add(1);
        }

        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_filter_with_verification_strict_mode() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        collection
            .insert("doc1", json!({ "name": "Alice", "age": 25 }))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({ "name": "Bob", "age": 30 }))
            .await
            .unwrap();

        let options = crate::VerificationOptions::strict();
        let mut stream = collection.filter_with_verification(
            |doc| {
                doc.data()
                    .get("age")
                    .and_then(|v| v.as_i64())
                    .map_or(false, |age| age >= 25)
            },
            &options,
        );

        use futures::StreamExt;
        let mut count = 0;
        while let Some(result) = stream.next().await {
            assert!(result.is_ok());
            count = count.saturating_add(1);
        }

        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_query_with_verification_strict_mode() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        collection
            .insert("doc1", json!({ "name": "Alice", "age": 25 }))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({ "name": "Bob", "age": 30 }))
            .await
            .unwrap();

        let options = crate::VerificationOptions::strict();
        let query = crate::QueryBuilder::new()
            .filter("age", crate::Operator::GreaterOrEqual, json!(25))
            .build();

        let result = collection.query_with_verification(query, &options).await;
        assert!(result.is_ok());

        use futures::StreamExt;
        let mut stream = result.unwrap().documents;
        let mut count = 0;
        while let Some(doc_result) = stream.next().await {
            assert!(doc_result.is_ok());
            count = count.saturating_add(1);
        }

        assert_eq!(count, 2);
    }
}
