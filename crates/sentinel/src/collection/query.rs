use async_stream::stream;
use futures::{StreamExt as _, TryStreamExt as _};
use serde_json::Value;
use tokio::fs as tokio_fs;
use tokio_stream::Stream;
use tracing::{debug, trace};

use crate::{
    comparison::compare_values,
    filtering::matches_filters,
    projection::project_document,
    streaming::stream_document_ids,
    Document,
    Result,
    SentinelError,
};
use super::coll::Collection;

#[allow(clippy::multiple_inherent_impl, reason = "multiple impl blocks for Collection are intentional for organization")]
impl Collection {
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

                        let collection_ref = Self {
                            path: collection_path.clone(),
                                                created_at: chrono::Utc::now(),
                                                updated_at: std::sync::RwLock::new(chrono::Utc::now()),
                                                last_checkpoint_at: std::sync::RwLock::new(None),
                                                total_documents: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
                                                total_size_bytes: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
                            signing_key: signing_key.clone(),
                                                stored_wal_config: sentinel_wal::CollectionWalConfig::default(),
                                                wal_manager: None,
                                                wal_config: sentinel_wal::CollectionWalConfig::default(),
                                                event_sender: None,
                                                event_task: None,
                            recovery_mode: std::sync::atomic::AtomicBool::new(false),
                        };

                        if let Err(e) = collection_ref.verify_document(&doc_with_id, &options).await {
                            if matches!(e, SentinelError::HashVerificationFailed { .. } | SentinelError::SignatureVerificationFailed { .. }) {
                                if options.hash_verification_mode == crate::VerificationMode::Strict
                                    || options.signature_verification_mode == crate::VerificationMode::Strict
                                    || options.empty_signature_mode == crate::VerificationMode::Strict
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
}
