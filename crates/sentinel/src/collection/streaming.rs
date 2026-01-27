use async_stream::stream;
use tokio::fs as tokio_fs;
use tokio_stream::Stream;
use tracing::{debug, trace};

use crate::{streaming::stream_document_ids, Document, Result, SentinelError};
use super::coll::Collection;

#[allow(clippy::multiple_inherent_impl, reason = "multiple impl blocks for Collection are intentional for organization")]
impl Collection {
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

                                            let collection_ref = Self {
                                                path: collection_path.clone(),
                                                created_at: chrono::Utc::now(),
                                                updated_at: std::sync::RwLock::new(chrono::Utc::now()),
                                                last_checkpoint_at: std::sync::RwLock::new(None),
                                                total_documents: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
                                                total_size_bytes: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
                                                signing_key: signing_key.clone(),
                                                wal_manager: None,
                                                stored_wal_config: sentinel_wal::CollectionWalConfig::default(),
                                                wal_config: sentinel_wal::CollectionWalConfig::default(),
                                                event_sender: None,
                                                event_task: None,
                                                recovery_mode: std::sync::atomic::AtomicBool::new(false),
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

                                            let collection_ref = Self {
                                                path: collection_path.clone(),
                                                created_at: chrono::Utc::now(),
                                                updated_at: std::sync::RwLock::new(chrono::Utc::now()),
                                                last_checkpoint_at: std::sync::RwLock::new(None),
                                                total_documents: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
                                                total_size_bytes: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
                                                signing_key: signing_key.clone(),
                                                wal_manager: None,
                                                stored_wal_config: sentinel_wal::CollectionWalConfig::default(),
                                                wal_config: sentinel_wal::CollectionWalConfig::default(),
                                                event_sender: None,
                                                event_task: None,
                                                recovery_mode: std::sync::atomic::AtomicBool::new(false),
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
}
