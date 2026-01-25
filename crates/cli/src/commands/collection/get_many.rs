use clap::Args;
use tracing::info;

/// Arguments for collection get-many command.
#[derive(Args)]
pub struct GetManyArgs {
    /// Document IDs to retrieve (can be used multiple times)
    #[arg(short, long = "id", value_name = "ID")]
    pub ids: Vec<String>,

    /// Output format: json or table
    #[arg(long, default_value = "json")]
    pub format: String,

    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal: crate::commands::WalArgs,
}

/// Execute collection get-many command.
///
/// Retrieves multiple documents from the specified collection by their IDs.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection_name` - Name of the collection
/// * `passphrase` - Optional passphrase for decrypting signing key
/// * `args` - Get-many command arguments
///
/// # Returns
/// Returns `Ok(())` on success.
pub async fn run(
    store_path: String,
    collection_name: String,
    passphrase: Option<String>,
    args: GetManyArgs,
) -> sentinel_dbms::Result<()> {
    if args.ids.is_empty() {
        info!("No document IDs specified");
        return Ok(());
    }

    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let collection = store
        .collection_with_config(&collection_name, Some(args.wal.to_overrides()))
        .await?;

    // Convert Vec<String> to Vec<&str>
    let ids: Vec<&str> = args.ids.iter().map(|s| s.as_str()).collect();

    let documents = collection.get_many(&ids).await?;

    match args.format.as_str() {
        "json" => {
            let results: Vec<serde_json::Value> = documents
                .into_iter()
                .zip(ids.iter())
                .map(|(doc, id)| {
                    if let Some(doc) = doc {
                        serde_json::json!({
                            "id": id,
                            "found": true,
                            "data": doc.data()
                        })
                    }
                    else {
                        serde_json::json!({
                            "id": id,
                            "found": false
                        })
                    }
                })
                .collect();

            println!("{}", serde_json::to_string_pretty(&results)?);
        },
        "table" => {
            println!("{:<30} {:<6} Data Preview", "ID", "Found");
            println!("{}", "-".repeat(80));

            for (doc, id) in documents.into_iter().zip(ids.iter()) {
                let found = if doc.is_some() { "Yes" } else { "No" };
                let preview = if let Some(doc) = &doc {
                    let data_str = serde_json::to_string(&doc.data()).unwrap();
                    if data_str.len() > 40 {
                        format!("{}...", &data_str[.. 37])
                    }
                    else {
                        data_str
                    }
                }
                else {
                    String::from("")
                };
                println!("{:<30} {:<6} {}", id, found, preview);
            }
        },
        _ => {
            return Err(sentinel_dbms::SentinelError::Internal {
                message: format!("Invalid format: {}. Use 'json' or 'table'", args.format),
            });
        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_get_many_existing_documents_json() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store and create collection with documents
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let docs = vec![
            ("doc1", serde_json::json!({"name": "Alice"})),
            ("doc2", serde_json::json!({"name": "Bob"})),
        ];
        collection.bulk_insert(docs).await.unwrap();

        let args = GetManyArgs {
            ids:    vec!["doc1".to_string(), "doc2".to_string()],
            format: "json".to_string(),
            wal:    crate::commands::WalArgs::default(),
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
    async fn test_get_many_mixed_documents_table() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store and create collection with one document
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let docs = vec![("doc1", serde_json::json!({"name": "Alice"}))];
        collection.bulk_insert(docs).await.unwrap();

        let args = GetManyArgs {
            ids:    vec!["doc1".to_string(), "doc2".to_string()],
            format: "table".to_string(),
            wal:    crate::commands::WalArgs::default(),
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
    async fn test_get_many_empty_ids() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store and create collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let args = GetManyArgs {
            ids:    vec![],
            format: "json".to_string(),
            wal:    crate::commands::WalArgs::default(),
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
    async fn test_get_many_invalid_format() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store and create collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let args = GetManyArgs {
            ids:    vec!["doc1".to_string()],
            format: "invalid".to_string(),
            wal:    crate::commands::WalArgs::default(),
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
    async fn test_get_many_non_existing_documents_json() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store and create collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let args = GetManyArgs {
            ids:    vec!["nonexistent1".to_string(), "nonexistent2".to_string()],
            format: "json".to_string(),
            wal:    crate::commands::WalArgs::default(),
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
    async fn test_get_many_non_existing_documents_table() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store and create collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let args = GetManyArgs {
            ids:    vec!["nonexistent1".to_string(), "nonexistent2".to_string()],
            format: "table".to_string(),
            wal:    crate::commands::WalArgs::default(),
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
    async fn test_get_many_invalid_store_path() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("invalid_store");
        std::fs::write(&store_path, "not a directory").unwrap();
        let collection_name = "test_collection";

        let args = GetManyArgs {
            ids:    vec!["doc1".to_string()],
            format: "json".to_string(),
            wal:    crate::commands::WalArgs::default(),
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
    async fn test_get_many_invalid_document_ids() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store and create collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let args = GetManyArgs {
            ids:    vec!["valid_id".to_string(), "invalid id with spaces".to_string()],
            format: "json".to_string(),
            wal:    crate::commands::WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        // Should handle invalid IDs gracefully (either error or skip them)
        // For now, we'll assume it errors on invalid IDs like other commands
        assert!(result.is_err());
    }
}
