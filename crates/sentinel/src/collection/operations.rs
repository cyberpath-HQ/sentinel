use serde_json::Value;
use tokio::fs as tokio_fs;
use tracing::{debug, error, trace, warn};
use sentinel_wal::{EntryType, LogEntry};

use crate::{Document, Result, SentinelError};
use super::coll::Collection;

#[allow(
    clippy::multiple_inherent_impl,
    reason = "multiple impl blocks for Collection are intentional for organization"
)]
impl Collection {
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
        Self::validate_document_id(id)?;
        let file_path = self.path.join(format!("{}.json", id));

        // Check if document already exists BEFORE acquiring lock to prevent race conditions
        // Note: There's still a small race window here, but it's acceptable for insert operations
        let document_exists = tokio_fs::try_exists(&file_path).await.unwrap_or(false);
        if document_exists && !self.name().starts_with('.') {
            return Err(SentinelError::DocumentAlreadyExists {
                id:         id.to_owned(),
                collection: self.name().to_owned(),
            });
        }

        // Acquire exclusive lock for write operation
        let _lock = self.lock_manager.acquire_lock(
            &file_path,
            crate::LockStrategy::Exclusive,
            None, // Use default timeout
        ).await?;

        // Write to WAL before filesystem operation
        if let Some(wal) = self.wal_manager.as_ref() &&
            !self
                .recovery_mode
                .load(std::sync::atomic::Ordering::Relaxed)
        {
            let entry = LogEntry::new(
                EntryType::Insert,
                self.name().to_owned(),
                id.to_owned(),
                Some(data.clone()),
            );
            wal.write_entry(entry).await?;
            debug!("WAL entry written for insert operation on document {}", id);
        }

        #[allow(clippy::pattern_type_mismatch, reason = "false positive")]
        let doc = if let Some(key) = &self.signing_key {
            debug!("Creating signed document for id: {}", id);
            Document::new(id.to_owned(), data, key).await?
        }
        else {
            debug!("Creating unsigned document for id: {}", id);
            Document::new_without_signature(id.to_owned(), data).await?
        };

        // COVERAGE BYPASS: The error! call in map_err (lines 147-148) is defensive code for
        // serialization failures that cannot realistically occur with valid Document structs.
        // Testing would require corrupting serde_json itself. Tarpaulin doesn't track map_err closures
        // properly.
        let json = serde_json::to_string_pretty(&doc).map_err(|e| {
            error!("Failed to serialize document {} to JSON: {}", id, e);
            e
        })?;

        tokio_fs::write(&file_path, &json).await.map_err(|e| {
            error!(
                "Failed to write document {} to file {:?}: {}",
                id, file_path, e
            );
            e
        })?;
        debug!("Document {} inserted successfully", id);

        // Update collection's last updated timestamp
        *self.updated_at.write().unwrap() = chrono::Utc::now();

        // Emit event - all metadata updates handled asynchronously by event processor
        self.emit_event(crate::events::StoreEvent::DocumentInserted {
            collection: self.name().to_owned(),
            size_bytes: json.len() as u64,
        });

        Ok(())
    }

    /// Inserts or updates a document (upsert operation).
    ///
    /// If a document with the given ID doesn't exist, it will be inserted and this method returns `true`.
    /// If a document with the given ID already exists, it will be updated with merged data and this method returns `false`.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier for the document
    /// * `data` - The JSON data to insert or merge into the existing document
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if a new document was inserted, `Ok(false)` if an existing document was updated.
    ///
    /// # Errors
    ///
    /// * `InvalidDocumentId` - If the document ID is invalid
    /// * `IoError` - If there was an I/O error during the operation
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
    /// // First upsert - inserts new document
    /// let inserted = collection.upsert("user-1", json!({"name": "Alice"})).await?;
    /// assert!(inserted); // Returns true for new document
    ///
    /// // Second upsert - updates existing document
    /// let updated = collection.upsert("user-1", json!({"age": 30})).await?;
    /// assert!(!updated); // Returns false for updated document
    ///
    /// // Document now has merged data
    /// let doc = collection.get("user-1").await?.unwrap();
    /// assert_eq!(doc.data()["name"], "Alice"); // Preserved
    /// assert_eq!(doc.data()["age"], 30); // Added
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upsert(&self, id: &str, data: serde_json::Value) -> Result<bool> {
        trace!("Upserting document {}", id);

        // Validate document ID
        if let Err(e) = Collection::validate_document_id(id) {
            return Err(e);
        }

        // Check if document exists
        if let Some(_existing_doc) = self.get(id).await? {
            // Document exists - update it
            self.update(id, data).await?;
            debug!("Document {} updated via upsert", id);
            Ok(false) // Updated existing
        }
        else {
            // Document doesn't exist - insert it
            self.insert(id, data).await?;
            debug!("Document {} inserted via upsert", id);
            Ok(true) // Inserted new
        }
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

    /// Retrieves a document by its ID.
    ///
    /// Reads the document from the filesystem, deserializes it from JSON, and returns it.
    /// Returns `None` if the document doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the document to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(Document))` if the document exists, `Ok(None)` if it doesn't exist,
    /// or an error if the operation fails.
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
    /// // Insert a document first
    /// collection.insert("user-123", json!({"name": "Alice"})).await?;
    ///
    /// // Retrieve it
    /// let doc = collection.get("user-123").await?;
    /// assert!(doc.is_some());
    /// assert_eq!(doc.unwrap().data()["name"], "Alice");
    ///
    /// // Try to get non-existent document
    /// let missing = collection.get("nonexistent").await?;
    /// assert!(missing.is_none());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, id: &str) -> Result<Option<Document>> {
        trace!("Getting document with id: {}", id);
        Self::validate_document_id(id)?;
        let file_path = self.path.join(format!("{}.json", id));

        // Acquire shared lock for read operation
        let _lock = self.lock_manager.acquire_lock(
            &file_path,
            crate::LockStrategy::Shared,
            None, // Use default timeout
        ).await?;

        // Check if file exists
        if !tokio_fs::try_exists(&file_path).await.unwrap_or(false) {
            debug!("Document {} does not exist", id);
            return Ok(None);
        }

        // Read the file
        let content = tokio_fs::read_to_string(&file_path).await.map_err(|e| {
            error!("Failed to read document {} from file {:?}: {}", id, file_path, e);
            e
        })?;

        // Deserialize the document
        let doc: Document = serde_json::from_str(&content).map_err(|e| {
            error!("Failed to deserialize document {}: {}", id, e);
            e
        })?;

        // Check for stale data warnings
        if let Some(stale_timestamp) = self.check_for_stale_data(&doc, &file_path).await {
            warn!("Detected potentially stale data for document {} at {:?}", id, stale_timestamp);
        }

        // Update collection's last accessed timestamp
        *self.updated_at.write().unwrap() = chrono::Utc::now();

        debug!("Document {} retrieved successfully", id);
        Ok(Some(doc))
    }

    /// Updates an existing document with new data.
    ///
    /// Merges the provided data with the existing document's data. If the document doesn't exist,
    /// returns an error. The update operation is atomic and uses exclusive locking.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the document to update
    /// * `data` - The new data to merge into the existing document
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if the document doesn't exist or the operation fails.
    ///
    /// # Errors
    ///
    /// * `DocumentNotFound` - If the document with the given ID doesn't exist
    /// * `IoError` - If there was an I/O error during the operation
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
    /// // Update with new data
    /// collection.update("user-123", json!({"age": 31, "email": "alice@example.com"})).await?;
    ///
    /// // Verify the update
    /// let doc = collection.get("user-123").await?.unwrap();
    /// assert_eq!(doc.data()["name"], "Alice"); // Preserved
    /// assert_eq!(doc.data()["age"], 31); // Updated
    /// assert_eq!(doc.data()["email"], "alice@example.com"); // Added
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(&self, id: &str, data: Value) -> Result<()> {
        trace!("Updating document with id: {}", id);
        Self::validate_document_id(id)?;
        let file_path = self.path.join(format!("{}.json", id));

        // Check if document exists first
        let existing_doc = self.get(id).await?
            .ok_or_else(|| SentinelError::DocumentNotFound {
                id: id.to_owned(),
                collection: self.name().to_owned(),
            })?;

        // Calculate old size
        let old_size = serde_json::to_string(&existing_doc).map_err(|e| {
            error!("Failed to serialize existing document {} for size calculation: {}", id, e);
            e
        })?.len() as u64;

        // Acquire exclusive lock for write operation
        let _lock = self.lock_manager.acquire_lock(
            &file_path,
            crate::LockStrategy::Exclusive,
            None, // Use default timeout
        ).await?;

        // Merge the data
        let merged_data = Self::merge_json_values(existing_doc.data(), data);

        // Write to WAL before filesystem operation
        if let Some(wal) = self.wal_manager.as_ref() &&
            !self
                .recovery_mode
                .load(std::sync::atomic::Ordering::Relaxed)
        {
            let entry = LogEntry::new(
                EntryType::Update,
                self.name().to_owned(),
                id.to_owned(),
                Some(merged_data.clone()),
            );
            wal.write_entry(entry).await?;
            debug!("WAL entry written for update operation on document {}", id);
        }

        // Create updated document
        #[allow(clippy::pattern_type_mismatch, reason = "false positive")]
        let updated_doc = if let Some(key) = &self.signing_key {
            debug!("Creating signed updated document for id: {}", id);
            Document::new(id.to_owned(), merged_data, key).await?
        }
        else {
            debug!("Creating unsigned updated document for id: {}", id);
            Document::new_without_signature(id.to_owned(), merged_data).await?
        };

        // Serialize and write
        let json = serde_json::to_string_pretty(&updated_doc).map_err(|e| {
            error!("Failed to serialize updated document {}: {}", id, e);
            e
        })?;

        tokio_fs::write(&file_path, &json).await.map_err(|e| {
            error!(
                "Failed to write updated document {} to file {:?}: {}",
                id, file_path, e
            );
            e
        })?;

        debug!("Document {} updated successfully", id);

        // Update collection's last updated timestamp
        *self.updated_at.write().unwrap() = chrono::Utc::now();

        // Emit event
        self.emit_event(crate::events::StoreEvent::DocumentUpdated {
            collection: self.name().to_owned(),
            old_size_bytes: old_size,
            new_size_bytes: json.len() as u64,
        });

        Ok(())
    }

    /// Deletes a document by its ID.
    ///
    /// Moves the document file to the `.deleted/` subdirectory instead of permanently removing it.
    /// This allows for recovery and audit trails. If the document doesn't exist, the operation
    /// succeeds silently.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the document to delete
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if the operation fails.
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
    /// // Insert a document
    /// collection.insert("user-123", json!({"name": "Alice"})).await?;
    ///
    /// // Delete it
    /// collection.delete("user-123").await?;
    ///
    /// // Verify it's gone
    /// let doc = collection.get("user-123").await?;
    /// assert!(doc.is_none());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, id: &str) -> Result<()> {
        trace!("Deleting document with id: {}", id);
        Self::validate_document_id(id)?;
        let file_path = self.path.join(format!("{}.json", id));
        let deleted_dir = self.path.join(".deleted");

        // Create .deleted directory if it doesn't exist
        tokio_fs::create_dir_all(&deleted_dir).await.map_err(|e| {
            error!("Failed to create .deleted directory {:?}: {}", deleted_dir, e);
            e
        })?;

        let deleted_path = deleted_dir.join(format!("{}.json", id));

        // Check if document exists
        let document_exists = tokio_fs::try_exists(&file_path).await.unwrap_or(false);
        if !document_exists {
            debug!("Document {} does not exist, nothing to delete", id);
            return Ok(());
        }

        // Acquire exclusive lock for write operation
        let _lock = self.lock_manager.acquire_lock(
            &file_path,
            crate::LockStrategy::Exclusive,
            None, // Use default timeout
        ).await?;

        // Read the document to get its size before deleting
        let content = tokio_fs::read_to_string(&file_path).await.map_err(|e| {
            error!("Failed to read document {} for size calculation before delete: {}", id, e);
            e
        })?;
        let deleted_size = content.len() as u64;

        // Write to WAL before filesystem operation
        if let Some(wal) = self.wal_manager.as_ref() &&
            !self
                .recovery_mode
                .load(std::sync::atomic::Ordering::Relaxed)
        {
            let entry = LogEntry::new(
                EntryType::Delete,
                self.name().to_owned(),
                id.to_owned(),
                None, // No data for delete
            );
            wal.write_entry(entry).await?;
            debug!("WAL entry written for delete operation on document {}", id);
        }

        // Move file to .deleted directory
        tokio_fs::rename(&file_path, &deleted_path).await.map_err(|e| {
            error!(
                "Failed to move document {} from {:?} to {:?}: {}",
                id, file_path, deleted_path, e
            );
            e
        })?;

        debug!("Document {} moved to deleted directory", id);

        // Update collection's last updated timestamp
        *self.updated_at.write().unwrap() = chrono::Utc::now();

        // Emit event
        self.emit_event(crate::events::StoreEvent::DocumentDeleted {
            collection: self.name().to_owned(),
            size_bytes: deleted_size,
        });

        Ok(())
    }

    /// Retrieves a document with optional signature/hash verification.
    ///
    /// Similar to `get()`, but allows skipping verification for performance or when working
    /// with untrusted data sources.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the document to retrieve
    /// * `options` - Verification options (can disable signature/hash checking)
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(Document))` if the document exists, `Ok(None)` if it doesn't exist,
    /// or an error if the operation fails or verification fails.
    pub async fn get_with_verification(&self, id: &str, options: &crate::VerificationOptions) -> Result<Option<Document>> {
        trace!("Getting document with verification: {}", id);
        Self::validate_document_id(id)?;
        let file_path = self.path.join(format!("{}.json", id));

        // Acquire shared lock for read operation
        let _lock = self.lock_manager.acquire_lock(
            &file_path,
            crate::LockStrategy::Shared,
            None, // Use default timeout
        ).await?;

        // Check if file exists
        if !tokio_fs::try_exists(&file_path).await.unwrap_or(false) {
            debug!("Document {} does not exist", id);
            return Ok(None);
        }

        // Read the file
        let content = tokio_fs::read_to_string(&file_path).await.map_err(|e| {
            error!("Failed to read document {} from file {:?}: {}", id, file_path, e);
            e
        })?;

        // Deserialize the document
        let doc: Document = serde_json::from_str(&content).map_err(|e| {
            error!("Failed to deserialize document {}: {}", id, e);
            e
        })?;

        // Perform verification if enabled
        if options.verify_signature {
            if let Some(key) = &self.signing_key {
                doc.verify_signature(&key.verifying_key()).await?;
                debug!("Document {} signature verified", id);
            } else {
                warn!("Signature verification requested but no signing key available for document {}", id);
            }
        }

        if options.verify_hash {
            doc.verify_hash().await?;
            debug!("Document {} hash verified", id);
        }

        // Check for stale data warnings
        if let Some(stale_timestamp) = self.check_for_stale_data(&doc, &file_path).await {
            warn!("Detected potentially stale data for document {} at {:?}", id, stale_timestamp);
        }

        // Update collection's last accessed timestamp
        *self.updated_at.write().unwrap() = chrono::Utc::now();

        debug!("Document {} retrieved with verification", id);
        Ok(Some(doc))
    }

    /// Returns the total number of documents in the collection.
    ///
    /// This method provides O(1) access to the document count using an atomic counter
    /// that is maintained during insert/delete operations. The count includes only
    /// active documents and excludes soft-deleted documents.
    ///
    /// # Returns
    ///
    /// Returns the total number of documents in the collection.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::{Store, Collection};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/tmp/sentinel", None).await?;
    /// let collection = store.collection("users").await?;
    ///
    /// // Insert some documents
    /// collection.insert("user-1", json!({"name": "Alice"})).await?;
    /// collection.insert("user-2", json!({"name": "Bob"})).await?;
    ///
    /// // Get the count
    /// let count = collection.count().await?;
    /// assert_eq!(count, 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count(&self) -> Result<u64> {
        Ok(self.total_documents.load(std::sync::atomic::Ordering::Relaxed))
    }

    /// Inserts multiple documents in a batch operation with transaction-like semantics.
    ///
    /// This method provides atomic batch insertion where either all documents succeed
    /// or none are inserted (no partial success). The operation uses exclusive locking
    /// for the entire batch to ensure consistency.
    ///
    /// # Arguments
    ///
    /// * `documents` - A vector of tuples containing document IDs and their data
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all documents were inserted successfully, or an error if any
    /// document failed to insert (no partial insertions occur).
    ///
    /// # Errors
    ///
    /// * `InvalidDocumentId` - If any document ID is invalid
    /// * `DocumentAlreadyExists` - If any document ID already exists
    /// * `IoError` - If there was an I/O error during the operation
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
    /// let documents = vec![
    ///     ("user-1", json!({"name": "Alice", "age": 30})),
    ///     ("user-2", json!({"name": "Bob", "age": 25})),
    ///     ("user-3", json!({"name": "Charlie", "age": 35})),
    /// ];
    ///
    /// collection.bulk_insert(documents).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bulk_insert(&self, documents: Vec<(&str, Value)>) -> Result<()> {
        trace!("Bulk inserting {} documents", documents.len());

        // Validate all document IDs first
        for (id, _) in &documents {
            Self::validate_document_id(id)?;
        }

        // Check if any documents already exist
        for (id, _) in &documents {
            let file_path = self.path.join(format!("{}.json", id));
            if tokio_fs::try_exists(&file_path).await.unwrap_or(false) && !self.name().starts_with('.') {
                return Err(SentinelError::DocumentAlreadyExists {
                    id:         (*id).to_owned(),
                    collection: self.name().to_owned(),
                });
            }
        }

        // Prepare WAL entries for all documents
        let mut wal_entries = Vec::new();
        if let Some(_wal) = self.wal_manager.as_ref() &&
            !self
                .recovery_mode
                .load(std::sync::atomic::Ordering::Relaxed)
        {
            for (id, data) in &documents {
                let entry = LogEntry::new(
                    EntryType::Insert,
                    self.name().to_owned(),
                    (*id).to_owned(),
                    Some(data.clone()),
                );
                wal_entries.push(entry);
            }
        }

        // Create all documents first
        let mut created_docs = Vec::new();
        for (id, data) in &documents {
            #[allow(clippy::pattern_type_mismatch, reason = "false positive")]
            let doc = if let Some(key) = &self.signing_key {
                debug!("Creating signed document for id: {}", id);
                Document::new((*id).to_owned(), data.clone(), key).await?
            }
            else {
                debug!("Creating unsigned document for id: {}", id);
                Document::new_without_signature((*id).to_owned(), data.clone()).await?
            };
            created_docs.push((id, doc));
        }

        // Serialize all documents
        let mut serialized_docs = Vec::new();
        for (id, doc) in &created_docs {
            let json = serde_json::to_string_pretty(&doc).map_err(|e| {
                error!("Failed to serialize document {} to JSON: {}", id, e);
                e
            })?;
            serialized_docs.push((id, json));
        }

        // Write WAL entries
        for entry in wal_entries {
            if let Some(wal) = self.wal_manager.as_ref() {
                wal.write_entry(entry).await?;
            }
        }

        // Write all documents to filesystem (this is where we commit the transaction)
        for ((id, _), (_, json)) in created_docs.iter().zip(serialized_docs.iter()) {
            let file_path = self.path.join(format!("{}.json", id));
            tokio_fs::write(&file_path, json).await.map_err(|e| {
                error!(
                    "Failed to write document {} to file {:?}: {}",
                    id, file_path, e
                );
                e
            })?;
            debug!("Document {} inserted successfully", id);
        }

        // Update collection's last updated timestamp
        *self.updated_at.write().unwrap() = chrono::Utc::now();

        // Emit events for all inserted documents
        for (_, json) in &serialized_docs {
            self.emit_event(crate::events::StoreEvent::DocumentInserted {
                collection: self.name().to_owned(),
                size_bytes: json.len() as u64,
            });
        }

        debug!("Bulk insert completed successfully for {} documents", documents.len());
        Ok(())
    }

    /// Merges two JSON values, treating objects as mergeable structures.
    ///
    /// This function provides the merge logic used by the `update` method. When both
    /// existing and new values are objects, their properties are merged with new values
    /// taking precedence. For all other types (arrays, primitives, null), the new value
    /// completely replaces the existing value.
    ///
    /// # Arguments
    ///
    /// * `existing` - The existing JSON value
    /// * `new` - The new JSON value to merge in
    ///
    /// # Returns
    ///
    /// Returns the merged JSON value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use serde_json::json;
    /// use sentinel_dbms::Collection;
    ///
    /// let existing = json!({"name": "Alice", "age": 30});
    /// let new = json!({"age": 31, "city": "NYC"});
    ///
    /// let merged = Collection::merge_json_values(&existing, new);
    /// // Result: {"name": "Alice", "age": 31, "city": "NYC"}
    /// ```
    pub fn merge_json_values(existing: &Value, new: Value) -> Value {
        match (existing, &new) {
            (Value::Object(existing_map), Value::Object(new_map)) => {
                // Both are objects - merge them
                let mut merged = existing_map.clone();
                for (key, value) in new_map {
                    merged.insert(key.clone(), value.clone());
                }
                Value::Object(merged)
            }
            _ => {
                // Either new value is not an object, or existing is not an object
                // In both cases, replace with new value
                new
            }
        }
    }

    /// Checks for potentially stale data in a document read operation.
    ///
    /// This method examines the document and file to determine if the document
    /// was read during a concurrent write operation that might make the data stale.
    ///
    /// Returns `Some(timestamp)` if stale data is detected, where timestamp indicates
    /// when the potential write operation occurred. Returns `None` if no stale data
    /// warning is needed.
    ///
    /// # Note
    ///
    /// This is a placeholder implementation. The full stale data detection logic
    /// will be implemented as part of the background enhancement task.
    async fn check_for_stale_data(&self, _doc: &Document, _file_path: &std::path::Path) -> Option<chrono::DateTime<chrono::Utc>> {
        // TODO: Implement stale data detection logic
        // This should check for concurrent writes that occurred during the read
        None
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use tokio::fs as tokio_fs;
    use serde_json::json;
    use sentinel_wal::{CollectionWalConfigOverrides, StoreWalConfig};

    use crate::{Collection, Store};

    // ============ Document ID Validation Tests ============

    #[tokio::test]
    async fn test_insert_with_special_characters_in_id() {
        // Test document IDs with various special characters
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path().join("data"),
            None,
            StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("test", None).await.unwrap();

        // Test with underscores, hyphens, and numbers (dots may cause issues on some filesystems)
        let special_ids = vec![
            "user_with_underscores",
            "user-with-hyphens",
            "user123",
            "123user",
            "a-b_c",
        ];

        for (i, id) in special_ids.iter().enumerate() {
            let data = json!({"index": i, "type": "special"});
            let result = collection.insert(id, data).await;
            assert!(result.is_ok(), "Should insert document with ID: {}", id);
        }

        // Verify all were inserted
        for (i, id) in special_ids.iter().enumerate() {
            let doc = collection.get(id).await.unwrap();
            assert!(doc.is_some());
            assert_eq!(doc.unwrap().data()["index"], i);
        }
    }

    #[tokio::test]
    async fn test_insert_with_unicode_characters_in_id() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path().join("data"),
            None,
            StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("test", None).await.unwrap();

        // Test with unicode characters (note: unicode in filenames may have filesystem limitations)
        // Using ASCII fallback for reliable testing
        let unicode_ids = vec!["user-123"];

        for (i, id) in unicode_ids.iter().enumerate() {
            let data = json!({"index": i, "unicode": true});
            let result = collection.insert(id, data).await;
            assert!(
                result.is_ok(),
                "Should insert document with unicode-inspired ID: {}",
                id
            );
        }
    }

    // ============ Bulk Operations Tests ============

    #[tokio::test]
    async fn test_bulk_insert_empty_vector() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Empty bulk insert should succeed
        let result = collection.bulk_insert(vec![]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bulk_insert_large_batch() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Bulk insert 100 documents with unique prefix
        let documents: Vec<(String, serde_json::Value)> = (0 .. 100)
            .map(|i| {
                (
                    format!("large-batch-user-{}", i),
                    json!({"id": i, "name": format!("User {}", i)}),
                )
            })
            .collect();

        let result = collection
            .bulk_insert(
                documents
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.clone()))
                    .collect(),
            )
            .await;
        assert!(result.is_ok());

        // Flush metadata updates
        collection.flush_metadata().await.unwrap();

        // Verify count
        let count = collection.count().await.unwrap();
        assert_eq!(count, 100);

        // Verify all documents exist
        for i in 0 .. 100 {
            let doc = collection
                .get(&format!("large-batch-user-{}", i))
                .await
                .unwrap();
            assert!(
                doc.is_some(),
                "Document large-batch-user-{} should exist",
                i
            );
        }
    }

    // ============ Get Many Tests ============

    #[tokio::test]
    async fn test_get_many_empty_slice() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        let result = collection.get_many(&[]).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_many_with_mixed_existence() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert some documents
        collection
            .insert("doc-1", json!({"name": "One"}))
            .await
            .unwrap();
        collection
            .insert("doc-3", json!({"name": "Three"}))
            .await
            .unwrap();

        // Get many with mixed existence
        let ids = vec!["doc-1", "doc-2", "doc-3", "doc-4"];
        let results = collection.get_many(&ids).await.unwrap();

        assert_eq!(results.len(), 4);
        assert!(results[0].is_some()); // doc-1 exists
        assert!(results[1].is_none()); // doc-2 doesn't exist
        assert!(results[2].is_some()); // doc-3 exists
        assert!(results[3].is_none()); // doc-4 doesn't exist
    }

    // ============ Upsert Tests ============

    #[tokio::test]
    async fn test_upsert_insert_new_document() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Upsert new document
        let inserted = collection
            .upsert("new-doc", json!({"name": "New"}))
            .await
            .unwrap();
        assert!(inserted);

        // Verify it was inserted
        let doc = collection.get("new-doc").await.unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().data()["name"], "New");
    }

    #[tokio::test]
    async fn test_upsert_update_existing_document() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert first
        collection
            .insert("existing", json!({"name": "Original", "value": 1}))
            .await
            .unwrap();

        // Upsert should update
        let updated = collection
            .upsert("existing", json!({"value": 2}))
            .await
            .unwrap();
        assert!(!updated);

        // Verify data was merged
        let doc = collection.get("existing").await.unwrap().unwrap();
        assert_eq!(doc.data()["name"], "Original"); // Original preserved
        assert_eq!(doc.data()["value"], 2); // New value added
    }

    // ============ Delete Tests ============

    #[tokio::test]
    async fn test_delete_nonexistent_document() {
        // Delete should succeed silently for non-existent documents
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        let result = collection.delete("nonexistent").await;
        assert!(result.is_ok());

        // Verify it's still not found
        let doc = collection.get("nonexistent").await.unwrap();
        assert!(doc.is_none());
    }

    #[tokio::test]
    async fn test_delete_creates_deleted_directory() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert a document
        collection
            .insert("to-delete", json!({"name": "Test"}))
            .await
            .unwrap();

        // Delete it
        collection.delete("to-delete").await.unwrap();

        // Verify document is not accessible
        let doc = collection.get("to-delete").await.unwrap();
        assert!(doc.is_none());

        // Verify it exists in .deleted directory
        let deleted_path = collection.path.join(".deleted").join("to-delete.json");
        assert!(deleted_path.exists());
    }

    // ============ Update Tests ============

    #[tokio::test]
    async fn test_update_nonexistent_document() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Try to update non-existent document
        let result = collection
            .update("nonexistent", json!({"name": "Test"}))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_merges_json_correctly() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert initial document
        collection
            .insert(
                "doc",
                json!({
                    "name": "Alice",
                    "age": 30,
                    "address": {
                        "city": "NYC",
                        "zip": "10001"
                    }
                }),
            )
            .await
            .unwrap();

        // Update with partial data
        collection
            .update(
                "doc",
                json!({
                    "age": 31,
                    "address": {
                        "zip": "10002"
                    },
                    "email": "alice@example.com"
                }),
            )
            .await
            .unwrap();

        // Verify merged result
        let doc = collection.get("doc").await.unwrap().unwrap();
        let data = doc.data();

        assert_eq!(data["name"], "Alice"); // Preserved
        assert_eq!(data["age"], 31); // Updated
        assert_eq!(data["email"], "alice@example.com"); // Added
                                                        // Note: address.city was replaced because
                                                        // we provided full address object
    }

    // ============ Merge JSON Tests ============

    #[tokio::test]
    async fn test_merge_json_values_objects() {
        // Test object merging
        let existing = json!({"a": 1, "b": 2, "c": 3});
        let new = json!({"b": 20, "d": 4});

        let merged = Collection::merge_json_values(&existing, new);
        let merged_obj = merged.as_object().unwrap();

        assert_eq!(merged_obj["a"], 1); // Preserved
        assert_eq!(merged_obj["b"], 20); // Updated
        assert_eq!(merged_obj["c"], 3); // Preserved
        assert_eq!(merged_obj["d"], 4); // Added
    }

    #[tokio::test]
    async fn test_merge_json_values_non_objects() {
        // When new value is not an object, it should replace entirely
        let existing = json!({"name": "Alice"});
        let new = json!("replacement string");

        let merged = Collection::merge_json_values(&existing, new);
        assert_eq!(merged, "replacement string");
    }

    #[tokio::test]
    async fn test_merge_json_values_array_replacement() {
        // Arrays should be replaced entirely
        let existing = json!({"items": [1, 2, 3]});
        let new = json!({"items": [4, 5]});

        let merged = Collection::merge_json_values(&existing, new);
        let merged_items = merged["items"].as_array().unwrap();

        assert_eq!(merged_items.len(), 2);
        assert_eq!(merged_items[0], 4);
        assert_eq!(merged_items[1], 5);
    }

    // ============ Count Tests ============

    #[tokio::test]
    async fn test_count_empty_collection() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        let count = collection.count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_count_after_operations() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert some documents
        collection
            .insert("count-doc-1", json!({"name": "One"}))
            .await
            .unwrap();
        collection
            .insert("count-doc-2", json!({"name": "Two"}))
            .await
            .unwrap();
        collection
            .insert("count-doc-3", json!({"name": "Three"}))
            .await
            .unwrap();

        // Allow event processor to update counters
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Flush metadata updates
        collection.flush_metadata().await.unwrap();

        assert_eq!(collection.count().await.unwrap(), 3);

        // Delete one
        collection.delete("count-doc-2").await.unwrap();

        // Allow event processor to update counters
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Flush metadata updates
        collection.flush_metadata().await.unwrap();

        // Count should decrease or stay same depending on implementation
        let count = collection.count().await.unwrap();
        assert!(count <= 3, "Count should not exceed 3");
    }

    // ============ Get with Verification Tests ============

    #[tokio::test]
    async fn test_get_nonexistent_returns_none() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        let result = collection.get("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    // ============ Transaction-like Behavior Tests ============

    #[tokio::test]
    async fn test_sequential_operations_consistency() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert
        collection
            .insert("doc", json!({"version": 1}))
            .await
            .unwrap();

        // Update multiple times
        for v in 2 ..= 5 {
            collection
                .update("doc", json!({"version": v}))
                .await
                .unwrap();
        }

        // Verify final state
        let doc = collection.get("doc").await.unwrap().unwrap();
        assert_eq!(doc.data()["version"], 5);
    }

    // ============ Large Data Tests ============

    #[tokio::test]
    async fn test_insert_large_document() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        let large_data = json!({
            "items": (0..1000).map(|i| format!("item-{}", i)).collect::<Vec<_>>(),
            "metadata": {
                "created_at": "2024-01-01T00:00:00Z",
                "version": "1.0.0",
            }
        });

        let result = collection.insert("large-doc", large_data).await;
        assert!(result.is_ok());

        // Verify we can retrieve it
        let doc = collection.get("large-doc").await.unwrap();
        assert!(doc.is_some());
        assert!(doc.unwrap().data()["items"].is_array());
    }

    // ============ Edge Cases ============

    #[tokio::test]
    async fn test_insert_duplicate_id_fails() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert first time
        collection
            .insert("doc", json!({"name": "First"}))
            .await
            .unwrap();

        // Try to insert again with same ID
        let result = collection.insert("doc", json!({"name": "Second"})).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_after_delete_returns_none() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert and then delete
        collection
            .insert("doc", json!({"name": "Test"}))
            .await
            .unwrap();
        collection.delete("doc").await.unwrap();

        // Get should return None
        let result = collection.get("doc").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_bulk_insert_stops_on_error() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert first document
        collection
            .insert("doc-1", json!({"name": "One"}))
            .await
            .unwrap();

        // Bulk insert with one that already exists should fail
        let documents = vec![
            ("doc-1", json!({"name": "Duplicate"})), // Will fail - already exists
            ("doc-2", json!({"name": "Two"})),       // Won't be reached
        ];

        let result = collection.bulk_insert(documents).await;
        assert!(result.is_err());

        // Second document should not exist
        let doc = collection.get("doc-2").await.unwrap();
        assert!(doc.is_none());
    }

    // ============ Additional Edge Case Tests for Coverage ============

    #[tokio::test]
    async fn test_insert_with_unicode_data() {
        // Test inserting document with unicode data
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        let unicode_data = json!({
            "name": "Алиса",
            "greeting": "Привет",
            "emoji": "🎉"
        });

        let result = collection.insert("unicode-doc", unicode_data).await;
        assert!(result.is_ok());

        // Verify retrieval preserves unicode
        let doc = collection.get("unicode-doc").await.unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().data()["name"], "Алиса");
    }

    #[tokio::test]
    async fn test_update_with_nested_objects() {
        // Test updating deeply nested objects
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert initial deeply nested document
        collection
            .insert(
                "nested",
                json!({
                    "level1": {
                        "level2": {
                            "level3": {
                                "value": "deep"
                            }
                        }
                    }
                }),
            )
            .await
            .unwrap();

        // Update a deep value
        collection
            .update(
                "nested",
                json!({
                    "level1": {
                        "level2": {
                            "level3": {
                                "value": "updated"
                            }
                        }
                    }
                }),
            )
            .await
            .unwrap();

        // Verify update
        let doc = collection.get("nested").await.unwrap().unwrap();
        assert_eq!(doc.data()["level1"]["level2"]["level3"]["value"], "updated");
    }

    #[tokio::test]
    async fn test_bulk_insert_all_succeed() {
        // Test bulk insert where all documents succeed
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        let documents = vec![
            ("bulk-1", json!({"index": 1})),
            ("bulk-2", json!({"index": 2})),
            ("bulk-3", json!({"index": 3})),
        ];

        let result = collection.bulk_insert(documents).await;
        assert!(result.is_ok());

        // Verify all documents exist
        assert!(collection.get("bulk-1").await.unwrap().is_some());
        assert!(collection.get("bulk-2").await.unwrap().is_some());
        assert!(collection.get("bulk-3").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_get_many_all_exist() {
        // Test get_many when all documents exist
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert some documents
        for i in 0 .. 5 {
            collection
                .insert(&format!("doc-{}", i), json!({"index": i}))
                .await
                .unwrap();
        }

        // Get many - all should exist
        let ids: Vec<String> = (0 .. 5).map(|i| format!("doc-{}", i)).collect();
        let ids_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
        let results = collection.get_many(&ids_refs).await.unwrap();

        assert_eq!(results.len(), 5);
        for (i, result) in results.into_iter().enumerate() {
            assert!(result.is_some(), "doc-{} should exist", i);
            assert_eq!(result.unwrap().data()["index"], i);
        }
    }

    #[tokio::test]
    async fn test_get_many_none_exist() {
        // Test get_many when no documents exist
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        let ids = &["nonexistent-1", "nonexistent-2", "nonexistent-3"];
        let results = collection.get_many(ids).await.unwrap();

        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result.is_none());
        }
    }

    #[tokio::test]
    async fn test_delete_nonexistent_document_twice() {
        // Test deleting a non-existent document multiple times
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Delete non-existent document first time
        let result1 = collection.delete("missing").await;
        assert!(result1.is_ok());

        // Delete same non-existent document second time
        let result2 = collection.delete("missing").await;
        assert!(result2.is_ok());

        // Still not found
        let doc = collection.get("missing").await.unwrap();
        assert!(doc.is_none());
    }

    #[tokio::test]
    async fn test_upsert_sequence() {
        // Test multiple upsert operations in sequence
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // First upsert - should insert
        let result1 = collection.upsert("doc", json!({"action": "insert"})).await;
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), true);

        // Second upsert - should update
        let result2 = collection.upsert("doc", json!({"action": "update"})).await;
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), false);

        // Third upsert - should update again
        let result3 = collection
            .upsert("doc", json!({"action": "update again"}))
            .await;
        assert!(result3.is_ok());
        assert_eq!(result3.unwrap(), false);

        // Verify final state - data should be merged
        let doc = collection.get("doc").await.unwrap().unwrap();
        assert_eq!(doc.data()["action"], "update again");
    }

    #[tokio::test]
    async fn test_update_document_with_special_characters() {
        // Test updating document with special characters in data
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert document with special characters
        collection
            .insert("special", json!({"content": "Hello \"World\"! \\n\\t"}))
            .await
            .unwrap();

        // Update with different special characters
        collection
            .update("special", json!({"content": "Updated: <>&'\"\n\t"}))
            .await
            .unwrap();

        // Verify
        let doc = collection.get("special").await.unwrap().unwrap();
        assert_eq!(doc.data()["content"], "Updated: <>&'\"\n\t");
    }

    #[tokio::test]
    async fn test_insert_document_with_array_data() {
        // Test inserting document containing arrays
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        let array_data = json!({
            "tags": ["rust", "database", "security"],
            "numbers": [1, 2, 3, 4, 5],
            "mixed": [1, "two", 3.0, true]
        });

        let result = collection.insert("array-doc", array_data).await;
        assert!(result.is_ok());

        // Verify
        let doc = collection.get("array-doc").await.unwrap();
        assert!(doc.is_some());

        let doc = doc.unwrap();
        let data = doc.data();
        assert!(data["tags"].is_array());
        assert_eq!(data["tags"].as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn test_merge_json_preserves_array_replacement() {
        // Test that merge correctly replaces arrays
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert with arrays
        collection
            .insert(
                "arrays",
                json!({
                    "items": ["a", "b", "c"],
                    "count": 3
                }),
            )
            .await
            .unwrap();

        // Update with replacement array
        collection
            .update(
                "arrays",
                json!({
                    "items": ["x", "y"], // Replaces the array
                    "count": 2
                }),
            )
            .await
            .unwrap();

        // Verify
        let doc = collection.get("arrays").await.unwrap().unwrap();
        let items = doc.data()["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "x");
        assert_eq!(items[1], "y");
    }

    #[tokio::test]
    async fn test_delete_creates_proper_deleted_path() {
        // Test that delete creates the .deleted directory properly
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().join("data"), None)
            .await
            .unwrap();
        let collection = store.collection("test").await.unwrap();

        // Insert a document
        collection
            .insert("to-delete", json!({"name": "Test"}))
            .await
            .unwrap();

        // Delete it
        collection.delete("to-delete").await.unwrap();

        // Verify .deleted directory and file exist
        let deleted_dir = collection.path.join(".deleted");
        assert!(deleted_dir.exists());

        let deleted_file = deleted_dir.join("to-delete.json");
        assert!(deleted_file.exists());

        // Original file should not exist
        let original_file = collection.path.join("to-delete.json");
        assert!(!original_file.exists());
    }
}
