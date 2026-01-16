use std::{path::PathBuf, sync::Arc};

use serde_json::Value;
use tokio::fs as tokio_fs;
use tracing::{debug, error, trace, warn};

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
/// use sentinel_dbms::{Store, Collection};
/// use serde_json::json;
///
/// # async fn example() -> sentinel_dbms::Result<()> {
/// // Create a store and get a collection
/// let store = Store::new("/path/to/data", None).await?;
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
            Document::new(id.to_owned(), data, key)?
        }
        else {
            debug!("Creating unsigned document for id: {}", id);
            Document::new_without_signature(id.to_owned(), data)?
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
        trace!("Retrieving document with id: {}", id);
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
    /// Scans the collection directory for JSON files and returns their IDs
    /// (filenames without the .json extension). This operation reads the directory
    /// contents and filters for valid document files, skipping hidden directories
    /// and metadata directories for optimization.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<String>)` containing all document IDs in the collection,
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
    /// // Insert some documents
    /// collection.insert("user-123", json!({"name": "Alice"})).await?;
    /// collection.insert("user-456", json!({"name": "Bob"})).await?;
    ///
    /// // List all documents
    /// let ids = collection.list().await?;
    /// assert_eq!(ids.len(), 2);
    /// assert!(ids.contains(&"user-123".to_string()));
    /// assert!(ids.contains(&"user-456".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self) -> Result<Vec<String>> {
        trace!("Listing documents in collection: {}", self.name());
        let mut entries = tokio_fs::read_dir(&self.path).await.map_err(|e| {
            error!("Failed to read collection directory {:?}: {}", self.path, e);
            e
        })?;
        let mut ids = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !entry.file_type().await?.is_dir() &&
                let Some(extension) = path.extension() &&
                extension == "json" &&
                let Some(file_stem) = path.file_stem() &&
                let Some(id) = file_stem.to_str()
            {
                ids.push(id.to_owned());
            }
            // Skip directories (optimization)
        }

        // Sort for consistent ordering
        ids.sort();
        debug!(
            "Found {} documents in collection {}",
            ids.len(),
            self.name()
        );
        trace!("Documents listed successfully");
        Ok(ids)
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
    /// # Arguments
    ///
    /// * `predicate` - A function that takes a `&Document` and returns `true` if the document
    ///   should be included in the results.
    ///
    /// # Returns
    ///
    /// Returns a vector of documents that match the predicate.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that takes a `&Document` and returns `true` if the document
    ///   should be included in the results.
    ///
    /// # Returns
    ///
    /// Returns a vector of documents that match the predicate.
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
    /// // Insert some test data
    /// collection.insert("user-1", json!({"name": "Alice", "age": 25})).await?;
    /// collection.insert("user-2", json!({"name": "Bob", "age": 30})).await?;
    ///
    /// // Filter for users older than 26
    /// let adults = collection.filter(|doc| {
    ///     doc.data().get("age")
    ///         .and_then(|v| v.as_i64())
    ///         .map_or(false, |age| age > 26)
    /// }).await?;
    ///
    /// assert_eq!(adults.len(), 1);
    /// assert_eq!(adults[0].id(), "user-2");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn filter<F>(&self, predicate: F) -> Result<Vec<Document>>
    where
        F: Fn(&Document) -> bool,
    {
        trace!("Filtering documents in collection: {}", self.name());
        let ids = self.list().await?;
        let mut results = Vec::new();

        for id in ids {
            // Load document only when needed
            if let Some(doc) = self.get(&id).await? {
                if predicate(&doc) {
                    results.push(doc);
                }
            }
        }

        debug!(
            "Filter completed, found {} matching documents",
            results.len()
        );
        Ok(results)
    }

    /// Executes a structured query against the collection.
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
    ///
    /// # Returns
    ///
    /// Returns a `QueryResult` containing the matching documents and metadata.
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
    /// assert_eq!(result.documents.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query(&self, query: crate::Query) -> Result<crate::QueryResult> {
        use std::time::Instant;
        let start_time = Instant::now();

        trace!("Executing query on collection: {}", self.name());

        // Get all document IDs
        let all_ids = self.list().await?;
        let total_collection_size = all_ids.len();

        // If we need sorting, we must collect all matching documents first
        // to determine sort order. Otherwise, we can stream and apply limit early.
        let final_docs = if query.sort.is_some() {
            self.execute_sorted_query(&all_ids, &query).await?
        }
        else {
            self.execute_streaming_query(&all_ids, &query).await?
        };

        let execution_time = start_time.elapsed();
        debug!(
            "Query completed in {:?}, returned {} documents out of {} total",
            execution_time,
            final_docs.len(),
            total_collection_size
        );

        Ok(crate::QueryResult {
            documents: final_docs,
            total_count: total_collection_size, // Note: this is approximate for streaming queries
            execution_time,
        })
    }

    /// Executes a query that requires sorting by collecting all matching documents first.
    async fn execute_sorted_query(&self, all_ids: &[String], query: &crate::Query) -> Result<Vec<Document>> {
        // For sorted queries, we need to collect all matching documents to sort them
        // But we can optimize by only keeping document IDs and sort values during filtering
        let mut matching_docs = Vec::new();

        for id in all_ids {
            if let Some(doc) = self.get(id).await? {
                if self.matches_filters(&doc, &query.filters) {
                    matching_docs.push(doc);
                }
            }
        }

        // Apply sorting
        if let Some((field, order)) = &query.sort {
            matching_docs.sort_by(|a, b| {
                let a_val = a.data().get(field);
                let b_val = b.data().get(field);
                match order {
                    crate::SortOrder::Ascending => self.compare_values(a_val, b_val),
                    crate::SortOrder::Descending => self.compare_values(b_val, a_val),
                }
            });
        }

        // Apply offset and limit
        let offset = query.offset.unwrap_or(0);
        let start_idx = offset.min(matching_docs.len());
        let end_idx = if let Some(limit) = query.limit {
            (start_idx + limit).min(matching_docs.len())
        }
        else {
            matching_docs.len()
        };

        // Apply projection to the final results
        let mut final_docs = Vec::new();
        for doc in matching_docs
            .into_iter()
            .skip(start_idx)
            .take(end_idx - start_idx)
        {
            let projected_doc = if let Some(ref fields) = query.projection {
                self.project_document(&doc, fields)
            }
            else {
                doc
            };
            final_docs.push(projected_doc);
        }

        Ok(final_docs)
    }

    /// Executes a query without sorting, allowing streaming with early limit application.
    async fn execute_streaming_query(&self, all_ids: &[String], query: &crate::Query) -> Result<Vec<Document>> {
        let mut results = Vec::new();
        let limit = query.limit.unwrap_or(usize::MAX);
        let offset = query.offset.unwrap_or(0);
        let mut collected = 0;

        for id in all_ids {
            if let Some(doc) = self.get(id).await? {
                if self.matches_filters(&doc, &query.filters) {
                    collected += 1;

                    // Skip documents before offset
                    if collected <= offset {
                        continue;
                    }

                    // Apply projection
                    let projected_doc = if let Some(ref fields) = query.projection {
                        self.project_document(&doc, fields)
                    }
                    else {
                        doc
                    };

                    results.push(projected_doc);

                    // Early termination if we've reached the limit
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }

        Ok(results)
    }

    /// Checks if a document matches all the given filters.
    fn matches_filters(&self, doc: &Document, filters: &[crate::Filter]) -> bool {
        for filter in filters {
            if !self.matches_filter(doc, filter) {
                return false;
            }
        }
        true
    }

    /// Checks if a document matches a single filter.
    fn matches_filter(&self, doc: &Document, filter: &crate::Filter) -> bool {
        match filter {
            &crate::Filter::Equals(ref field, ref value) => self.get_field_value(doc, field) == Some(value),
            &crate::Filter::GreaterThan(ref field, ref value) => {
                self.compare_field_value(doc, field, value, |ord| ord == std::cmp::Ordering::Greater)
            },
            &crate::Filter::LessThan(ref field, ref value) => {
                self.compare_field_value(doc, field, value, |ord| ord == std::cmp::Ordering::Less)
            },
            &crate::Filter::GreaterOrEqual(ref field, ref value) => {
                self.compare_field_value(doc, field, value, |ord| ord != std::cmp::Ordering::Less)
            },
            &crate::Filter::LessOrEqual(ref field, ref value) => {
                self.compare_field_value(doc, field, value, |ord| ord != std::cmp::Ordering::Greater)
            },
            &crate::Filter::Contains(ref field, ref substring) => {
                self.get_field_value(doc, field)
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| s.contains(substring))
            },
            &crate::Filter::StartsWith(ref field, ref prefix) => {
                self.get_field_value(doc, field)
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| s.starts_with(prefix))
            },
            &crate::Filter::EndsWith(ref field, ref suffix) => {
                self.get_field_value(doc, field)
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| s.ends_with(suffix))
            },
            &crate::Filter::In(ref field, ref values) => {
                self.get_field_value(doc, field)
                    .is_some_and(|v| values.contains(v))
            },
            &crate::Filter::Exists(ref field, exists) => {
                let field_exists = doc.data().get(field).is_some();
                field_exists == exists
            },
            &crate::Filter::And(ref left, ref right) => {
                self.matches_filter(doc, left) && self.matches_filter(doc, right)
            },
            &crate::Filter::Or(ref left, ref right) => {
                self.matches_filter(doc, left) || self.matches_filter(doc, right)
            },
        }
    }

    /// Gets the value of a field from a document.
    fn get_field_value<'a>(&self, doc: &'a Document, field: &str) -> Option<&'a Value> { doc.data().get(field) }

    /// Compares a field value with a given value using a comparison function.
    fn compare_field_value<F>(&self, doc: &Document, field: &str, value: &Value, cmp: F) -> bool
    where
        F: Fn(std::cmp::Ordering) -> bool,
    {
        self.get_field_value(doc, field)
            .map(|field_val| cmp(self.compare_json_values(field_val, value)))
            .unwrap_or(false)
    }

    /// Compares two values for sorting purposes.
    fn compare_values(&self, a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
        match (a, b) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(va), Some(vb)) => self.compare_json_values(va, vb),
        }
    }

    /// Compares two JSON values for sorting.
    fn compare_json_values(&self, a: &Value, b: &Value) -> std::cmp::Ordering {
        match (a, b) {
            (&Value::Null, &Value::Null) => std::cmp::Ordering::Equal,
            (&Value::Null, _) => std::cmp::Ordering::Less,
            (_, &Value::Null) => std::cmp::Ordering::Greater,
            (&Value::Bool(ba), &Value::Bool(bb)) => ba.cmp(&bb),
            (&Value::Bool(_), _) => std::cmp::Ordering::Less,
            (_, &Value::Bool(_)) => std::cmp::Ordering::Greater,
            (&Value::Number(ref na), &Value::Number(ref nb)) => {
                // Compare as f64 for simplicity, may lose precision
                let fa = na.as_f64().unwrap_or(0.0);
                let fb = nb.as_f64().unwrap_or(0.0);
                fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
            },
            (&Value::Number(_), _) => std::cmp::Ordering::Less,
            (_, &Value::Number(_)) => std::cmp::Ordering::Greater,
            (&Value::String(ref sa), &Value::String(ref sb)) => sa.cmp(sb),
            (&Value::String(_), _) => std::cmp::Ordering::Less,
            (_, &Value::String(_)) => std::cmp::Ordering::Greater,
            (&Value::Array(ref aa), &Value::Array(ref ab)) => aa.len().cmp(&ab.len()), // Simple length comparison
            (&Value::Array(_), _) => std::cmp::Ordering::Less,
            (_, &Value::Array(_)) => std::cmp::Ordering::Greater,
            (&Value::Object(ref oa), &Value::Object(ref ob)) => oa.len().cmp(&ob.len()), // Simple length comparison
        }
    }

    /// Projects a document to include only specified fields.
    fn project_document(&self, doc: &Document, fields: &[String]) -> Document {
        let mut projected_data = serde_json::Map::new();
        for field in fields {
            if let Some(value) = doc.data().get(field) {
                projected_data.insert(field.clone(), value.clone());
            }
        }
        // Create a new document with projected data
        // Note: This creates a new document without proper metadata/signing
        // For full implementation, we'd need to handle metadata properly
        Document::new_without_signature(doc.id().to_owned(), Value::Object(projected_data))
            .unwrap_or_else(|_| doc.clone())
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

    use super::*;
    use crate::Store;

    /// Helper function to set up a temporary collection for testing
    async fn setup_collection() -> (Collection, tempfile::TempDir) {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();
        (collection, temp_dir)
    }

    /// Helper function to set up a temporary collection with signing key for testing
    async fn setup_collection_with_signing_key() -> (Collection, tempfile::TempDir) {
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

        let ids = collection.list().await.unwrap();
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

        let ids = collection.list().await.unwrap();
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

        let ids = collection.list().await.unwrap();
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

        let ids = collection.list().await.unwrap();
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

        let ids = collection.list().await.unwrap();
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
        let ids = collection.list().await.unwrap();
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
    async fn test_filter_empty_collection() {
        let (collection, _temp_dir) = setup_collection().await;

        let results = collection.filter(|_| true).await.unwrap();
        assert!(results.is_empty());
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

        let results = collection.filter(|_| true).await.unwrap();
        assert_eq!(results.len(), 2);
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

        let results = collection
            .filter(|doc| {
                doc.data()
                    .get("age")
                    .and_then(|v| v.as_i64())
                    .map_or(false, |age| age > 26)
            })
            .await
            .unwrap();

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
        assert!(result.documents.is_empty());
        assert_eq!(result.total_count, 0);
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
        assert_eq!(result.documents.len(), 2);
        assert_eq!(result.total_count, 3);
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
        assert_eq!(result.documents.len(), 3);
        assert_eq!(result.documents[0].data()["name"], "Charlie");
        assert_eq!(result.documents[1].data()["name"], "Alice");
        assert_eq!(result.documents[2].data()["name"], "Bob");
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
        assert_eq!(result.documents.len(), 2);
        assert_eq!(result.total_count, 5);
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
        assert_eq!(result.documents.len(), 1);
        let doc = &result.documents[0];
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
        assert_eq!(result.documents.len(), 1);
        assert_eq!(result.documents[0].data()["name"], "Alice");
    }

    #[tokio::test]
    async fn test_query_logical_filters() {
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert(
                "user-1",
                json!({"name": "Alice", "age": 25, "active": true}),
            )
            .await
            .unwrap();
        collection
            .insert("user-2", json!({"name": "Bob", "age": 30, "active": false}))
            .await
            .unwrap();
        collection
            .insert(
                "user-3",
                json!({"name": "Charlie", "age": 35, "active": true}),
            )
            .await
            .unwrap();

        // Test AND logic (implicit in multiple filters)
        let query = crate::QueryBuilder::new()
            .filter("age", crate::Operator::GreaterThan, json!(26))
            .filter("active", crate::Operator::Equals, json!(true))
            .build();

        let result = collection.query(query).await.unwrap();
        assert_eq!(result.documents.len(), 1);
        assert_eq!(result.documents[0].data()["name"], "Charlie");
    }
}
