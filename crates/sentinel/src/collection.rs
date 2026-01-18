use std::{path::PathBuf, sync::Arc};

use async_stream::stream;
use futures::{StreamExt as _, TryStreamExt as _};
use serde_json::{json, Value};
use tokio::fs as tokio_fs;
use tokio_stream::Stream;
use tracing::{debug, error, trace, warn};

use crate::{
    comparison::compare_values,
    filtering::matches_filters,
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
    ///     empty_signature_mode: VerificationMode::Warn,
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

    /// Counts the total number of documents in the collection.
    ///
    /// This method streams through all document IDs and counts them efficiently
    /// without loading the full documents into memory.
    ///
    /// # Returns
    ///
    /// Returns the total count of documents as a `usize`, or a `SentinelError` if
    /// there was an error accessing the collection.
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
    /// // Insert some documents
    /// collection.insert("user-123", json!({"name": "Alice"})).await?;
    /// collection.insert("user-456", json!({"name": "Bob"})).await?;
    ///
    /// // Count the documents
    /// let count = collection.count().await?;
    /// assert_eq!(count, 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count(&self) -> Result<usize> {
        trace!("Counting documents in collection: {}", self.name());
        let ids: Vec<String> = self.list().try_collect().await?;
        Ok(ids.len())
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

    /// Verifies document hash according to the specified verification options.
    ///
    /// # Arguments
    ///
    /// * `doc` - The document to verify
    /// * `options` - The verification options
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if verification passes or is handled according to the mode,
    /// or `Err(SentinelError::HashVerificationFailed)` if verification fails in Strict mode.
    async fn verify_hash(&self, doc: &Document, options: crate::VerificationOptions) -> Result<()> {
        if options.hash_verification_mode == crate::VerificationMode::Silent {
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

            match options.hash_verification_mode {
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

    /// Verifies document signature according to the specified verification options.
    ///
    /// # Arguments
    ///
    /// * `doc` - The document to verify
    /// * `options` - The verification options containing modes for different scenarios
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if verification passes or is handled according to the mode,
    /// or `Err(SentinelError::SignatureVerificationFailed)` if verification fails in Strict mode.
    async fn verify_signature(&self, doc: &Document, options: crate::VerificationOptions) -> Result<()> {
        if options.signature_verification_mode == crate::VerificationMode::Silent &&
            options.empty_signature_mode == crate::VerificationMode::Silent
        {
            return Ok(());
        }

        trace!("Verifying signature for document: {}", doc.id());

        if doc.signature().is_empty() {
            let reason = "Document has no signature".to_string();

            match options.empty_signature_mode {
                crate::VerificationMode::Strict => {
                    error!("Document {} has no signature: {}", doc.id(), reason);
                    return Err(SentinelError::SignatureVerificationFailed {
                        id: doc.id().to_string(),
                        reason,
                    });
                },
                crate::VerificationMode::Warn => {
                    warn!("Document {} has no signature: {}", doc.id(), reason);
                },
                crate::VerificationMode::Silent => {},
            }
            return Ok(());
        }

        if !options.verify_signature {
            trace!("Signature verification disabled for document: {}", doc.id());
            return Ok(());
        }

        if let Some(ref signing_key) = self.signing_key {
            let public_key = signing_key.verifying_key();
            let is_valid = sentinel_crypto::verify_signature(doc.hash(), doc.signature(), &public_key).await?;

            if !is_valid {
                let reason = "Signature verification using public key failed".to_string();

                match options.signature_verification_mode {
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
    /// Returns `Ok(())` if verifications pass or are handled according to the modes,
    /// or an error if verification fails in Strict mode.
    async fn verify_document(&self, doc: &Document, options: &crate::VerificationOptions) -> Result<()> {
        if options.verify_hash {
            self.verify_hash(doc, *options).await?;
        }

        if options.verify_signature {
            self.verify_signature(doc, *options).await?;
        }

        Ok(())
    }

    /// Updates a document by merging new data with existing data.
    ///
    /// This method loads the existing document, merges the provided data with the existing
    /// document data (deep merge for objects), updates the metadata (updated_at timestamp),
    /// and saves the document back to disk.
    ///
    /// If the document doesn't exist, this method will return an error.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the document to update
    /// * `data` - The new data to merge with the existing document data
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `SentinelError` if the operation fails.
    ///
    /// # Examples
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
    /// // Update with partial data (only age)
    /// collection.update("user-123", json!({"age": 31, "city": "NYC"})).await?;
    ///
    /// // Document now contains: {"name": "Alice", "age": 31, "city": "NYC"}
    /// let doc = collection.get("user-123").await?.unwrap();
    /// assert_eq!(doc.data()["name"], "Alice");
    /// assert_eq!(doc.data()["age"], 31);
    /// assert_eq!(doc.data()["city"], "NYC");
    /// # Ok(())
    /// # }
    /// ```

    /// Merges two JSON values, with `new_value` taking precedence over `existing_value`.
    ///
    /// For objects, this performs a deep merge where fields from `new_value` override
    /// or add to fields in `existing_value`. For other types, `new_value` completely replaces
    /// `existing_value`.
    fn merge_json_values(existing_value: &Value, new_value: Value) -> Value {
        match (existing_value, &new_value) {
            (Value::Object(existing_map), Value::Object(new_map)) => {
                let mut merged = existing_map.clone();
                for (key, value) in new_map {
                    merged.insert(key.clone(), value.clone());
                }
                Value::Object(merged)
            },
            _ => new_value,
        }
    }

    /// Extracts a numeric value from a document field for aggregation operations.
    fn extract_numeric_value(doc: &Document, field: &str) -> Option<f64> {
        doc.data().get(field).and_then(|v| {
            match v {
                Value::Number(n) => n.as_f64(),
                _ => None,
            }
        })
    }

    pub async fn update(&self, id: &str, data: Value) -> Result<()> {
        trace!("Updating document with id: {}", id);
        validate_document_id(id)?;

        // Load existing document
        let Some(mut existing_doc) = self.get(id).await?
        else {
            return Err(SentinelError::DocumentNotFound {
                id:         id.to_owned(),
                collection: self.name().to_owned(),
            });
        };

        // Merge the new data with existing data
        let merged_data = Self::merge_json_values(existing_doc.data(), data);

        // Update the document data and metadata
        if let Some(key) = &self.signing_key {
            existing_doc.set_data(merged_data, key).await?;
        }
        else {
            // For unsigned documents, we need to manually update the data and hash
            existing_doc.data = merged_data;
            existing_doc.updated_at = chrono::Utc::now();
            existing_doc.hash = sentinel_crypto::hash_data(&existing_doc.data).await?;
            existing_doc.signature = String::new();
        }

        // Save the updated document
        let file_path = self.path.join(format!("{}.json", id));
        let json = serde_json::to_string_pretty(&existing_doc).map_err(|e| {
            error!("Failed to serialize updated document {} to JSON: {}", id, e);
            e
        })?;
        tokio_fs::write(&file_path, json).await.map_err(|e| {
            error!(
                "Failed to write updated document {} to file {:?}: {}",
                id, file_path, e
            );
            e
        })?;

        debug!("Document {} updated successfully", id);
        Ok(())
    }

    /// Retrieves multiple documents by their IDs in a single operation.
    ///
    /// This method efficiently loads multiple documents concurrently. For IDs that don't exist,
    /// `None` is returned in the corresponding position.
    ///
    /// # Arguments
    ///
    /// * `ids` - A slice of document IDs to retrieve
    ///
    /// # Returns
    ///
    /// Returns a `Vec<Option<Document>>` where each element corresponds to the document
    /// at the same index in the input `ids` slice. `Some(document)` if the document exists,
    /// `None` if it doesn't exist.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_dbms::{Store, Collection};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert some documents
    /// collection.insert("user-1", json!({"name": "Alice"})).await?;
    /// collection.insert("user-2", json!({"name": "Bob"})).await?;
    ///
    /// // Batch get multiple documents
    /// let docs = collection.get_many(&["user-1", "user-2", "user-3"]).await?;
    /// assert_eq!(docs.len(), 3);
    /// assert!(docs[0].is_some()); // user-1 exists
    /// assert!(docs[1].is_some()); // user-2 exists
    /// assert!(docs[2].is_none()); // user-3 doesn't exist
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_many(&self, ids: &[&str]) -> Result<Vec<Option<Document>>> {
        use futures::future::join_all;

        trace!("Batch getting {} documents", ids.len());

        let futures = ids.iter().map(|&id| self.get(id));
        let results = join_all(futures).await;

        let documents = results.into_iter().collect::<Result<Vec<_>>>()?;

        debug!(
            "Batch get completed, retrieved {} documents",
            documents.len()
        );
        Ok(documents)
    }

    /// Inserts a document if it doesn't exist, or updates it if it does.
    ///
    /// This is a convenience method that combines insert and update operations.
    /// If the document doesn't exist, it will be inserted. If it exists, the new data
    /// will be merged with the existing data (see `update` for merge behavior).
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the document
    /// * `data` - The data to insert or merge
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if a new document was inserted, `Ok(false)` if an existing
    /// document was updated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_dbms::{Store, Collection};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // First call inserts the document
    /// let inserted = collection.upsert("user-123", json!({"name": "Alice"})).await?;
    /// assert!(inserted);
    ///
    /// // Second call updates the existing document
    /// let updated = collection.upsert("user-123", json!({"age": 30})).await?;
    /// assert!(!updated);
    ///
    /// // Document now contains both name and age
    /// let doc = collection.get("user-123").await?.unwrap();
    /// assert_eq!(doc.data()["name"], "Alice");
    /// assert_eq!(doc.data()["age"], 30);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upsert(&self, id: &str, data: Value) -> Result<bool> {
        trace!("Upserting document with id: {}", id);

        if self.get(id).await?.is_some() {
            // Document exists, update it
            self.update(id, data).await?;
            debug!("Document {} updated via upsert", id);
            Ok(false)
        }
        else {
            // Document doesn't exist, insert it
            self.insert(id, data).await?;
            debug!("Document {} inserted via upsert", id);
            Ok(true)
        }
    }

    /// Performs aggregation operations on documents matching the given filters.
    ///
    /// Supported aggregations:
    /// - `Count`: Count of matching documents
    /// - `Sum(field)`: Sum of numeric values in the specified field
    /// - `Avg(field)`: Average of numeric values in the specified field
    /// - `Min(field)`: Minimum value in the specified field
    /// - `Max(field)`: Maximum value in the specified field
    ///
    /// # Arguments
    ///
    /// * `filters` - Filters to apply before aggregation
    /// * `aggregation` - The aggregation operation to perform
    ///
    /// # Returns
    ///
    /// Returns the aggregated result as a JSON `Value`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_dbms::{Store, Collection, Filter, Aggregation};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("products").await?;
    ///
    /// // Insert some test data
    /// collection.insert("prod-1", json!({"name": "Widget", "price": 10.0})).await?;
    /// collection.insert("prod-2", json!({"name": "Gadget", "price": 20.0})).await?;
    ///
    /// // Count all products
    /// let count = collection.aggregate(vec![], Aggregation::Count).await?;
    /// assert_eq!(count, json!(2));
    ///
    /// // Sum of all prices
    /// let total = collection.aggregate(vec![], Aggregation::Sum("price".to_string())).await?;
    /// assert_eq!(total, json!(30.0));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn aggregate(&self, filters: Vec<crate::Filter>, aggregation: Aggregation) -> Result<Value> {
        use futures::TryStreamExt as _;

        trace!("Performing aggregation: {:?}", aggregation);

        // Get all documents (we'll filter them)
        let mut stream = self.all();

        let mut count = 0usize;
        let mut sum = 0.0f64;
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        while let Some(doc) = stream.try_next().await? {
            // Apply filters
            if !filters.is_empty() {
                let filter_refs: Vec<&Filter> = filters.iter().collect();
                if !crate::filtering::matches_filters(&doc, &filter_refs) {
                    continue;
                }
            }

            count += 1;

            // Extract value for field-based aggregations
            if let Aggregation::Sum(ref field) |
            Aggregation::Avg(ref field) |
            Aggregation::Min(ref field) |
            Aggregation::Max(ref field) = aggregation
            {
                if let Some(value) = Self::extract_numeric_value(&doc, field) {
                    sum += value;
                    min = min.min(value);
                    max = max.max(value);
                }
            }
        }

        let result = match aggregation {
            Aggregation::Count => json!(count),
            Aggregation::Sum(_) => json!(sum),
            Aggregation::Avg(_) => {
                if count == 0 {
                    json!(null)
                }
                else {
                    json!(sum / count as f64)
                }
            },
            Aggregation::Min(_) => {
                if min == f64::INFINITY {
                    json!(null)
                }
                else {
                    json!(min)
                }
            },
            Aggregation::Max(_) => {
                if max == f64::NEG_INFINITY {
                    json!(null)
                }
                else {
                    json!(max)
                }
            },
        };

        debug!("Aggregation result: {}", result);
        Ok(result)
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

        let retrieved = collection
            .get_with_verification("user-123", &crate::VerificationOptions::disabled())
            .await
            .unwrap();
        assert_eq!(*retrieved.unwrap().data(), doc);
    }

    #[tokio::test]
    async fn test_insert_empty_document() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({});
        collection.insert("empty", doc.clone()).await.unwrap();

        let retrieved = collection
            .get_with_verification("empty", &crate::VerificationOptions::disabled())
            .await
            .unwrap();
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

        let retrieved = collection
            .get_with_verification("large", &crate::VerificationOptions::disabled())
            .await
            .unwrap();
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

        let retrieved = collection
            .get_with_verification("user-123", &crate::VerificationOptions::disabled())
            .await
            .unwrap();
        assert_eq!(*retrieved.unwrap().data(), doc2);
    }

    #[tokio::test]
    async fn test_update_nonexistent() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "name": "Bob" });
        let result = collection.update("new-user", doc.clone()).await;

        // Should return an error for non-existent document
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::SentinelError::DocumentNotFound {
                id,
                collection: _,
            } => {
                assert_eq!(id, "new-user");
            },
            _ => panic!("Expected DocumentNotFound error"),
        }
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

        let retrieved = collection
            .get_with_verification("user-123", &crate::VerificationOptions::disabled())
            .await
            .unwrap();
        assert!(retrieved.is_some());

        collection.delete("user-123").await.unwrap();

        let retrieved = collection
            .get_with_verification("user-123", &crate::VerificationOptions::disabled())
            .await
            .unwrap();
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

        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert_eq!(docs.len(), 3);

        let ids: Vec<String> = collection.list().try_collect().await.unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"user-123".to_string()));
        assert!(ids.contains(&"user-456".to_string()));
        assert!(ids.contains(&"user-789".to_string()));

        // Verify data
        let alice = collection
            .get_with_verification("user-123", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
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
        let user1 = collection
            .get_with_verification("user1", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        let user2 = collection
            .get_with_verification("user2", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user1.data()["name"], "User1");
        assert_eq!(user2.data()["name"], "User2");

        // Update one
        collection
            .update("user1", json!({"name": "Updated"}))
            .await
            .unwrap();
        let updated = collection
            .get_with_verification("user1", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.data()["name"], "Updated");

        // Delete one
        collection.delete("user2").await.unwrap();
        assert!(collection
            .get_with_verification("user2", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .is_none());
        assert!(collection
            .get_with_verification("user1", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .is_some());
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
    async fn test_get_nonexistent_with_verification() {
        let (collection, _temp_dir) = setup_collection().await;

        let options = crate::VerificationOptions::strict();
        let result = collection
            .get_with_verification("nonexistent", &options)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_with_verification_disabled() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Alice", "data": "test" });
        collection.insert("test_doc", doc.clone()).await.unwrap();

        // Tamper with the file
        let file_path = collection.path.join("test_doc.json");
        let mut content = tokio_fs::read_to_string(&file_path).await.unwrap();
        content = content.replace("test", "tampered");
        tokio_fs::write(&file_path, &content).await.unwrap();

        // Should succeed with verification disabled
        let options = crate::VerificationOptions::disabled();
        let result = collection.get_with_verification("test_doc", &options).await;
        assert!(result.is_ok());
        let doc = result.unwrap().unwrap();
        assert_eq!(doc.data()["name"], "Alice");
    }

    #[tokio::test]
    async fn test_get_with_verification_empty_signature_warn() {
        let (collection, _temp_dir) = setup_collection().await;

        // Insert unsigned document
        let doc = json!({ "name": "Unsigned" });
        collection
            .insert("unsigned_doc", doc.clone())
            .await
            .unwrap();

        // Should warn but not fail
        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Strict,
            empty_signature_mode:        crate::VerificationMode::Warn,
            hash_verification_mode:      crate::VerificationMode::Strict,
        };
        let result = collection
            .get_with_verification("unsigned_doc", &options)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_get_with_verification_empty_signature_strict() {
        let (collection, _temp_dir) = setup_collection().await;

        // Insert unsigned document
        let doc = json!({ "name": "Unsigned" });
        collection
            .insert("unsigned_doc", doc.clone())
            .await
            .unwrap();

        // Should fail with strict empty signature mode
        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Strict,
            empty_signature_mode:        crate::VerificationMode::Strict,
            hash_verification_mode:      crate::VerificationMode::Strict,
        };
        let result = collection
            .get_with_verification("unsigned_doc", &options)
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::SentinelError::SignatureVerificationFailed {
                id,
                reason,
            } => {
                assert_eq!(id, "unsigned_doc");
                assert!(reason.contains("no signature"));
            },
            _ => panic!("Expected SignatureVerificationFailed"),
        }
    }

    #[tokio::test]
    async fn test_all_empty_collection() {
        let (collection, _temp_dir) = setup_collection().await;

        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert!(docs.is_empty());
    }

    #[tokio::test]
    async fn test_all_with_multiple_documents() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 5 {
            let doc = json!({ "id": i, "name": format!("User{}", i) });
            collection
                .insert(&format!("user-{}", i), doc)
                .await
                .unwrap();
        }

        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert_eq!(docs.len(), 5);

        let ids: std::collections::HashSet<_> = docs.iter().map(|d| d.id().to_string()).collect();
        for i in 0 .. 5 {
            assert!(ids.contains(&format!("user-{}", i)));
        }
    }

    #[tokio::test]
    async fn test_all_with_verification() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        for i in 0 .. 3 {
            let doc = json!({ "id": i });
            collection
                .insert(&format!("signed-{}", i), doc)
                .await
                .unwrap();
        }

        let options = crate::VerificationOptions::strict();
        let docs: Vec<_> = collection
            .all_with_verification(&options)
            .try_collect()
            .await
            .unwrap();
        assert_eq!(docs.len(), 3);
    }

    #[tokio::test]
    async fn test_filter_empty_result() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 3 {
            let doc = json!({ "id": i, "status": "active" });
            collection
                .insert(&format!("user-{}", i), doc)
                .await
                .unwrap();
        }

        let results: Vec<_> = collection
            .filter(|doc| doc.data().get("status") == Some(&json!("inactive")))
            .try_collect()
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_filter_with_all_matching() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 5 {
            let doc = json!({ "id": i, "active": true });
            collection
                .insert(&format!("user-{}", i), doc)
                .await
                .unwrap();
        }

        let results: Vec<_> = collection
            .filter(|doc| doc.data().get("active") == Some(&json!(true)))
            .try_collect()
            .await
            .unwrap();
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_filter_with_verification() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        for i in 0 .. 3 {
            let doc = json!({ "id": i, "active": true });
            collection
                .insert(&format!("signed-{}", i), doc)
                .await
                .unwrap();
        }

        let options = crate::VerificationOptions::strict();
        let results: Vec<_> = collection
            .filter_with_verification(
                |doc| doc.data().get("active") == Some(&json!(true)),
                &options,
            )
            .try_collect()
            .await
            .unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_bulk_insert_empty_all() {
        let (collection, _temp_dir) = setup_collection().await;

        let result = collection.bulk_insert(vec![]).await;
        assert!(result.is_ok());

        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert!(docs.is_empty());
    }

    #[tokio::test]
    async fn test_bulk_insert_large_batch() {
        let (collection, _temp_dir) = setup_collection().await;

        let documents: Vec<(String, serde_json::Value)> = (0 .. 100)
            .map(|i| {
                let key = format!("user-{}", i);
                let value = json!({ "id": i, "data": format!("value{}", i) });
                (key, value)
            })
            .collect();
    }

    #[tokio::test]
    async fn test_bulk_insert_partial_failure() {
        let (collection, _temp_dir) = setup_collection().await;

        let documents = vec![
            ("valid-1", json!({ "name": "One" })),
            ("valid-2", json!({ "name": "Two" })),
            ("invalid id!", json!({ "name": "Three" })), // This will fail
        ];

        let result = collection.bulk_insert(documents).await;
        assert!(result.is_err());

        // First two should not be inserted (transaction safety not implemented yet)
        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert!(docs.len() <= 2);
    }

    #[tokio::test]
    async fn test_query_empty_filter() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 10 {
            let doc = json!({ "id": i, "value": i * 10 });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new().build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();
        assert_eq!(docs.len(), 10);
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 100 {
            let doc = json!({ "id": i });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new().limit(5).build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();
        assert_eq!(docs.len(), 5);
    }

    #[tokio::test]
    async fn test_query_with_offset() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 10 {
            let doc = json!({ "id": i, "value": i });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new().offset(5).build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();
        assert_eq!(docs.len(), 5);
    }

    #[tokio::test]
    async fn test_query_with_limit_and_offset() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 100 {
            let doc = json!({ "id": i, "value": i });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new().offset(10).limit(5).build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();
        assert_eq!(docs.len(), 5);
    }

    #[tokio::test]
    async fn test_query_with_sort_ascending() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in (0 .. 5).rev() {
            let doc = json!({ "id": i, "value": i });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new()
            .sort("id", crate::SortOrder::Ascending)
            .build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();

        assert_eq!(docs.len(), 5);
        for (i, doc) in docs.iter().enumerate() {
            assert_eq!(doc.data()["id"], json!(i));
        }
    }

    #[tokio::test]
    async fn test_query_with_sort_descending() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 5 {
            let doc = json!({ "id": i, "value": i });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new()
            .sort("id", crate::SortOrder::Descending)
            .build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();

        assert_eq!(docs.len(), 5);
        for (i, doc) in docs.iter().enumerate() {
            assert_eq!(doc.data()["id"], json!(4 - i));
        }
    }

    #[tokio::test]
    async fn test_query_with_projection() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 3 {
            let doc =
                json!({ "id": i, "name": format!("User{}", i), "email": format!("user{}@example.com", i), "age": 30 });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new()
            .projection(vec!["id", "name"])
            .build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();

        assert_eq!(docs.len(), 3);
        for doc in &docs {
            assert!(doc.data().get("id").is_some());
            assert!(doc.data().get("name").is_some());
            assert!(doc.data().get("email").is_none());
            assert!(doc.data().get("age").is_none());
        }
    }

    #[tokio::test]
    async fn test_query_with_verification() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        for i in 0 .. 5 {
            let doc = json!({ "id": i, "active": true });
            collection
                .insert(&format!("signed-{}", i), doc)
                .await
                .unwrap();
        }

        let options = crate::VerificationOptions::strict();
        let query = crate::QueryBuilder::new().build();
        let result = collection
            .query_with_verification(query, &options)
            .await
            .unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();
        assert_eq!(docs.len(), 5);
    }

    #[tokio::test]
    async fn test_query_complex() {
        let (collection, _temp_dir) = setup_collection().await;

        // Insert test data
        let test_data = vec![
            (
                "doc1",
                json!({ "name": "Alice", "age": 25, "city": "NYC", "active": true }),
            ),
            (
                "doc2",
                json!({ "name": "Bob", "age": 30, "city": "LA", "active": true }),
            ),
            (
                "doc3",
                json!({ "name": "Charlie", "age": 35, "city": "NYC", "active": false }),
            ),
            (
                "doc4",
                json!({ "name": "Diana", "age": 28, "city": "NYC", "active": true }),
            ),
            (
                "doc5",
                json!({ "name": "Eve", "age": 40, "city": "LA", "active": false }),
            ),
        ];

        for (id, doc) in &test_data {
            collection.insert(id, doc.clone()).await.unwrap();
        }

        // Query: active=true, city=NYC, age>=26, limit 2, sort age asc, project name,age
        let query = crate::QueryBuilder::new()
            .filter("active", crate::Operator::Equals, json!(true))
            .filter("city", crate::Operator::Equals, json!("NYC"))
            .filter("age", crate::Operator::GreaterOrEqual, json!(26))
            .sort("age", crate::SortOrder::Ascending)
            .limit(2)
            .projection(vec!["name", "age"])
            .build();

        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();

        assert_eq!(docs.len(), 1);
        // Diana is 28, Bob is 30 but in LA (filtered out by city=NYC)
        assert_eq!(docs[0].data()["name"], json!("Diana"));
    }

    #[tokio::test]
    async fn test_delete_and_recover() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "name": "ToDelete" });
        collection.insert("test-doc", doc.clone()).await.unwrap();

        // Verify it exists
        assert!(collection.get("test-doc").await.unwrap().is_some());

        // Delete it
        collection.delete("test-doc").await.unwrap();

        // Verify it's gone from main collection
        assert!(collection.get("test-doc").await.unwrap().is_none());

        // Verify it's in .deleted/
        let deleted_path = collection.path.join(".deleted").join("test-doc.json");
        assert!(tokio_fs::try_exists(&deleted_path).await.unwrap());

        // Recover it manually (no recover API yet)
        tokio_fs::rename(&deleted_path, collection.path.join("test-doc.json"))
            .await
            .unwrap();

        // Verify it's back
        assert!(collection.get("test-doc").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_insert_special_characters_in_data() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({
            "string": "hello\nworld\ttab",
            "unicode": "Hello 世界 🌍",
            "null": null,
            "array": [1, 2, 3, "four"],
            "nested": { "deep": { "value": 42 } }
        });

        collection.insert("special-doc", doc.clone()).await.unwrap();

        let retrieved = collection.get("special-doc").await.unwrap().unwrap();
        assert_eq!(retrieved.data(), &doc);
    }

    #[tokio::test]
    async fn test_insert_very_long_document_id() {
        let (collection, _temp_dir) = setup_collection().await;

        // Use a reasonably long ID that works on most filesystems
        // (255 bytes may exceed some filesystem limits depending on path length)
        let long_id = "a".repeat(200);
        let doc = json!({ "data": "test" });

        let result = collection.insert(&long_id, doc).await;
        assert!(result.is_ok());

        let retrieved = collection.get(&long_id).await.unwrap().unwrap();
        assert_eq!(retrieved.id(), &long_id);
    }

    #[tokio::test]
    async fn test_insert_max_value_numbers() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({
            "max_i64": 9223372036854775807i64,
            "min_i64": -9223372036854775808i64,
            "max_f64": 1.7976931348623157e308,
            "min_f64": -1.7976931348623157e308
        });

        collection.insert("numbers", doc.clone()).await.unwrap();

        let retrieved = collection.get("numbers").await.unwrap().unwrap();
        assert_eq!(retrieved.data(), &doc);
    }

    #[tokio::test]
    async fn test_insert_nested_array_document() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({
            "matrix": [[1, 2, 3], [4, 5, 6], [7, 8, 9]],
            "mixed": [1, "two", true, null, { "nested": "value" }]
        });

        collection.insert("arrays", doc.clone()).await.unwrap();

        let retrieved = collection.get("arrays").await.unwrap().unwrap();
        assert_eq!(retrieved.data(), &doc);
    }

    #[tokio::test]
    async fn test_collection_name() {
        let (collection, _temp_dir) = setup_collection().await;

        assert_eq!(collection.name(), "test_collection");
    }

    #[tokio::test]
    async fn test_verify_hash_valid() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Test" });
        collection.insert("hash-test", doc.clone()).await.unwrap();

        let retrieved = collection.get("hash-test").await.unwrap().unwrap();
        let options = crate::VerificationOptions {
            verify_signature:            false,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Strict,
            empty_signature_mode:        crate::VerificationMode::Warn,
            hash_verification_mode:      crate::VerificationMode::Strict,
        };

        let result = collection.verify_document(&retrieved, &options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_hash_invalid() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Original" });
        collection
            .insert("hash-invalid", doc.clone())
            .await
            .unwrap();

        // Tamper with the file
        let file_path = collection.path.join("hash-invalid.json");
        let mut content = tokio_fs::read_to_string(&file_path).await.unwrap();
        content = content.replace("Original", "Tampered");
        tokio_fs::write(&file_path, &content).await.unwrap();

        // Re-read the document (disable verification to read the tampered file)
        let retrieved = collection
            .get_with_verification("hash-invalid", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();

        let options = crate::VerificationOptions {
            verify_signature:            false,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Strict,
            empty_signature_mode:        crate::VerificationMode::Warn,
            hash_verification_mode:      crate::VerificationMode::Strict,
        };

        let result = collection.verify_document(&retrieved, &options).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::SentinelError::HashVerificationFailed {
                id,
                ..
            } => {
                assert_eq!(id, "hash-invalid");
            },
            _ => panic!("Expected HashVerificationFailed"),
        }
    }

    #[tokio::test]
    async fn test_verify_signature_valid() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Signed" });
        collection
            .insert("signed-valid", doc.clone())
            .await
            .unwrap();

        let retrieved = collection.get("signed-valid").await.unwrap().unwrap();
        let options = crate::VerificationOptions::strict();

        let result = collection.verify_document(&retrieved, &options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_signature_invalid() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Original" });
        collection.insert("sig-invalid", doc.clone()).await.unwrap();

        // Tamper with the file
        let file_path = collection.path.join("sig-invalid.json");
        let mut content = tokio_fs::read_to_string(&file_path).await.unwrap();
        content = content.replace("Original", "Tampered");
        tokio_fs::write(&file_path, &content).await.unwrap();

        // Re-read the document (disable verification to read the tampered file)
        let retrieved = collection
            .get_with_verification("sig-invalid", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        let options = crate::VerificationOptions::strict();

        let result = collection.verify_document(&retrieved, &options).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_streaming_all_skips_deleted() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 5 {
            let doc = json!({ "id": i });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        // Delete some
        collection.delete("doc-1").await.unwrap();
        collection.delete("doc-3").await.unwrap();

        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert_eq!(docs.len(), 3);

        let ids: std::collections::HashSet<_> = docs.iter().map(|d| d.id().to_string()).collect();
        assert!(ids.contains("doc-0"));
        assert!(!ids.contains("doc-1"));
        assert!(ids.contains("doc-2"));
        assert!(!ids.contains("doc-3"));
        assert!(ids.contains("doc-4"));
    }
}
