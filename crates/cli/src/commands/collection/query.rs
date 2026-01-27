use std::str::FromStr as _;

use clap::Args;
use sentinel_dbms::{futures::StreamExt as _, Filter, QueryBuilder, SortOrder, VerificationMode, VerificationOptions};
use serde_json::Value;
use tracing::{error, info};

use crate::commands::WalArgs;

/// Arguments for collection query command.
#[derive(Args, Clone, Default)]
pub struct QueryArgs {
    /// Verify document signature (default: true)
    #[arg(long, default_value = "true")]
    pub verify_signature: bool,
    /// Verify document hash (default: true)
    #[arg(long, default_value = "true")]
    pub verify_hash:      bool,
    /// Signature verification mode: strict, warn, or silent (default: strict)
    #[arg(long, default_value = "strict")]
    pub signature_mode:   String,
    /// How to handle documents with no signature: strict, warn, or silent (default: warn)
    #[arg(long, default_value = "warn")]
    pub empty_sig_mode:   String,
    /// Hash verification mode: strict, warn, or silent (default: strict)
    #[arg(long, default_value = "strict")]
    pub hash_mode:        String,
    /// Filter documents (can be used multiple times)
    /// Syntax: field=value, field>value, field<value, field>=value, field<=value,
    /// field~substring, field^prefix, field$suffix, field in:value1,value2, field exists:true/false
    #[arg(long, value_name = "filter")]
    pub filter:           Vec<String>,
    /// Sort by field (field:asc or field:desc)
    #[arg(long, value_name = "field:order")]
    pub sort:             Option<String>,
    /// Limit number of results
    #[arg(long)]
    pub limit:            Option<usize>,
    /// Skip number of results
    #[arg(long)]
    pub offset:           Option<usize>,
    /// Project fields (comma-separated)
    #[arg(long, value_name = "field1,field2")]
    pub project:          Option<String>,
    /// Output format: json or table
    #[arg(long, default_value = "json")]
    pub format:           String,
    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal:              WalArgs,
}

impl QueryArgs {
    /// Convert CLI arguments to verification options.
    fn to_verification_options(&self) -> Result<VerificationOptions, String> {
        let signature_verification_mode = VerificationMode::from_str(&self.signature_mode).map_err(|_e| {
            format!(
                "Invalid signature verification mode: {}",
                self.signature_mode
            )
        })?;

        let empty_signature_mode = VerificationMode::from_str(&self.empty_sig_mode)
            .map_err(|_e| format!("Invalid empty signature mode: {}", self.empty_sig_mode))?;

        let hash_verification_mode = VerificationMode::from_str(&self.hash_mode)
            .map_err(|_e| format!("Invalid hash verification mode: {}", self.hash_mode))?;

        Ok(VerificationOptions {
            verify_signature: self.verify_signature,
            verify_hash: self.verify_hash,
            signature_verification_mode,
            empty_signature_mode,
            hash_verification_mode,
        })
    }
}

/// Query documents in a Sentinel collection with filters and sorting.
///
/// This function allows complex querying with filters, sorting, pagination, and projection.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection` - Collection name
/// * `passphrase` - Optional passphrase for decrypting the signing key
/// * `args` - The parsed command-line arguments for collection query.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::collection::query::{run, QueryArgs};
///
/// let args = QueryArgs {
///     filter: vec![String::from("age>18")],
///     ..Default::default()
/// };
/// run(
///     String::from("/tmp/my_store"),
///     String::from("users"),
///     None,
///     args,
/// )
/// .await?;
/// ```
#[allow(clippy::print_stdout, reason = "CLI command output")]
pub async fn run(
    store_path: String,
    collection: String,
    passphrase: Option<String>,
    args: QueryArgs,
) -> sentinel_dbms::Result<()> {
    info!(
        "Querying documents in collection '{}' in store {}",
        collection, store_path
    );

    // Parse verification options
    let verification_options = args.to_verification_options().map_err(|e| {
        sentinel_dbms::SentinelError::ConfigError {
            message: e,
        }
    })?;

    // Parse filters
    let parsed_filters = parse_filters(&args.filter)?;

    // Parse sort
    let mut query_builder = QueryBuilder::new();
    if let Some(sort_str) = args.sort.as_deref() {
        if let Some((field, order)) = sort_str.split_once(':') {
            let sort_order = match order.to_lowercase().as_str() {
                "asc" => SortOrder::Ascending,
                "desc" => SortOrder::Descending,
                _ => {
                    return Err(sentinel_dbms::SentinelError::Internal {
                        message: format!("Invalid sort order: {}. Use asc or desc", order),
                    });
                },
            };
            query_builder = query_builder.sort(field, sort_order);
        }
        else {
            return Err(sentinel_dbms::SentinelError::Internal {
                message: format!(
                    "Invalid sort format: {}. Use field:asc or field:desc",
                    sort_str
                ),
            });
        }
    }

    // Set limit and offset
    if let Some(limit) = args.limit {
        query_builder = query_builder.limit(limit);
    }
    if let Some(offset) = args.offset {
        query_builder = query_builder.offset(offset);
    }

    // Parse projection
    if let Some(project_str) = args.project.as_deref() {
        let fields: Vec<&str> = project_str.split(',').map(|s| s.trim()).collect();
        query_builder = query_builder.projection(fields);
    }

    let mut query = query_builder.build();
    query.filters = parsed_filters;

    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let coll = store
        .collection_with_config(&collection, Some(args.wal.to_overrides()))
        .await?;

    // Use query instead of streaming all
    let result = coll
        .query_with_verification(query, &verification_options)
        .await?;

    // Output documents from the stream
    let mut count = 0usize;
    let mut stream = result.documents;
    while let Some(item) = stream.next().await {
        match item {
            Ok(doc) => {
                #[allow(clippy::print_stdout, reason = "CLI output")]
                {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(doc.data()).unwrap_or_else(|_| "{}".to_owned())
                    );
                }
                count = count.checked_add(1).unwrap_or(count);
            },
            Err(e) => {
                error!(
                    "Failed to read document in collection '{}' in store {}: {}",
                    collection, store_path, e
                );
                return Err(e);
            },
        }
    }

    info!("Found {} documents in collection '{}'", count, collection);
    Ok(())
}

/// Parse filter specifications from strings.
fn parse_filters(filter_specs: &[String]) -> sentinel_dbms::Result<Vec<Filter>> {
    let mut filters = Vec::new();

    for spec in filter_specs {
        let filter = parse_filter(spec)?;
        filters.push(filter);
    }

    Ok(filters)
}

/// Parse a single filter specification.
fn parse_filter(spec: &str) -> sentinel_dbms::Result<Filter> {
    // Split on first occurrence of operator
    let operators = [
        "==", "!=", ">=", "<=", ">", "<", "~", "^", "$", " in:", " exists:",
    ];

    for op in &operators {
        if let Some((field, value_str)) = spec.split_once(op) {
            let field = field.trim().to_owned();

            match *op {
                "==" => {
                    let value = parse_value(value_str.trim())?;
                    return Ok(Filter::Equals(field, value));
                },
                "!=" => {
                    return Err(sentinel_dbms::SentinelError::Internal {
                        message: "Not equals operator (!=) is not supported".to_owned(),
                    });
                },
                ">=" => {
                    let value = parse_value(value_str.trim())?;
                    return Ok(Filter::GreaterOrEqual(field, value));
                },
                "<=" => {
                    let value = parse_value(value_str.trim())?;
                    return Ok(Filter::LessOrEqual(field, value));
                },
                ">" => {
                    let value = parse_value(value_str.trim())?;
                    return Ok(Filter::GreaterThan(field, value));
                },
                "<" => {
                    let value = parse_value(value_str.trim())?;
                    return Ok(Filter::LessThan(field, value));
                },
                "~" => {
                    return Ok(Filter::Contains(field, value_str.trim().to_owned()));
                },
                "^" => {
                    return Ok(Filter::StartsWith(field, value_str.trim().to_owned()));
                },
                "$" => {
                    return Ok(Filter::EndsWith(field, value_str.trim().to_owned()));
                },
                " in:" => {
                    let values = parse_value_list(value_str.trim())?;
                    return Ok(Filter::In(field, values));
                },
                " exists:" => {
                    let exists = parse_bool(value_str.trim())?;
                    return Ok(Filter::Exists(field, exists));
                },
                _ => {},
            }
        }
    }

    // Default to equals if no operator found
    if let Some((field, value_str)) = spec.split_once('=') {
        let value = parse_value(value_str.trim())?;
        return Ok(Filter::Equals(field.trim().to_owned(), value));
    }

    Err(sentinel_dbms::SentinelError::Internal {
        message: format!(
            "Invalid filter: {}. Use field=value, field>value, etc.",
            spec
        ),
    })
}

/// Parse a JSON value from string.
fn parse_value(s: &str) -> sentinel_dbms::Result<Value> {
    // Try to parse as JSON first
    if let Ok(value) = serde_json::from_str(s) {
        return Ok(value);
    }

    // If not valid JSON, treat as string
    Ok(Value::String(s.to_owned()))
}

/// Parse a list of values from comma-separated string.
fn parse_value_list(s: &str) -> sentinel_dbms::Result<Vec<Value>> {
    let mut values = Vec::new();
    for item in s.split(',') {
        let value = parse_value(item.trim())?;
        values.push(value);
    }
    Ok(values)
}

/// Parse boolean from string.
fn parse_bool(s: &str) -> sentinel_dbms::Result<bool> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => {
            Err(sentinel_dbms::SentinelError::Internal {
                message: format!("Invalid boolean: {}. Use true or false", s),
            })
        },
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn test_query_empty_collection() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");
        let collection_name = "test_collection";

        // Create store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        // Run query command
        let args = QueryArgs {
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
            filter:           vec![],
            sort:             None,
            limit:            None,
            offset:           None,
            project:          None,
            format:           "json".to_string(),
            wal:              WalArgs::default(),
        };
        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_populated_collection() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");
        let collection_name = "test_collection";

        // Create store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        let collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        // Insert some documents
        collection
            .insert("doc1", serde_json::json!({"name": "Alice", "age": 30}))
            .await
            .unwrap();
        collection
            .insert("doc2", serde_json::json!({"name": "Bob", "age": 25}))
            .await
            .unwrap();

        // Allow event processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Run query command
        let args = QueryArgs {
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
            filter:           vec![],
            sort:             None,
            limit:            None,
            offset:           None,
            project:          None,
            format:           "json".to_string(),
            wal:              WalArgs::default(),
        };
        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");
        let collection_name = "test_collection";

        // Create store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        let collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        // Insert some documents
        collection
            .insert("doc1", serde_json::json!({"name": "Alice"}))
            .await
            .unwrap();
        collection
            .insert("doc2", serde_json::json!({"name": "Bob"}))
            .await
            .unwrap();
        collection
            .insert("doc3", serde_json::json!({"name": "Charlie"}))
            .await
            .unwrap();

        // Allow event processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Run query command with limit
        let args = QueryArgs {
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
            filter:           vec![],
            sort:             None,
            limit:            Some(2),
            offset:           None,
            project:          None,
            format:           "json".to_string(),
            wal:              WalArgs::default(),
        };
        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_invalid_verification_mode() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");
        let collection_name = "test_collection";

        // Create store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        // Run query command with invalid verification mode
        let args = QueryArgs {
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "invalid".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
            filter:           vec![],
            sort:             None,
            limit:            None,
            offset:           None,
            project:          None,
            format:           "json".to_string(),
            wal:              WalArgs::default(),
        };
        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_with_corrupted_documents_strict_verification() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");
        let collection_name = "test_collection";
        let passphrase = "test_passphrase";

        // Create store and collection with passphrase
        let store = sentinel_dbms::Store::new_with_config(
            &store_path,
            Some(passphrase),
            sentinel_dbms::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        // Insert a valid document first
        let data = serde_json::json!({"name": "test", "value": 42});
        collection.insert("doc1", data).await.unwrap();

        // Manually corrupt the document file by changing its signature
        let doc_path = store_path
            .join("data")
            .join(collection_name)
            .join("doc1.json");
        let mut content: serde_json::Value =
            serde_json::from_str(&tokio::fs::read_to_string(&doc_path).await.unwrap()).unwrap();

        // Corrupt the signature
        if let Some(signature) = content.get_mut("signature") {
            *signature = serde_json::json!("corrupted_signature");
        }

        tokio::fs::write(&doc_path, serde_json::to_string_pretty(&content).unwrap())
            .await
            .unwrap();

        // Run query command with strict verification and passphrase
        let args = QueryArgs {
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "strict".to_string(),
            hash_mode:        "strict".to_string(),
            filter:           vec![],
            sort:             None,
            limit:            None,
            offset:           None,
            project:          None,
            format:           "json".to_string(),
            wal:              WalArgs::default(),
        };
        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            Some(passphrase.to_string()),
            args,
        )
        .await;

        // Should fail due to corrupted document signature
        assert!(result.is_err());
    }
}
