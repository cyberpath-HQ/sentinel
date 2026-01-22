use clap::Args;
use tracing::info;

/// Arguments for collection count command.
#[derive(Args)]
pub struct CountArgs {
    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal: crate::commands::WalArgs,
}

/// Execute collection count command.
///
/// Counts the number of documents in the specified collection.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection_name` - Name of the collection
/// * `passphrase` - Optional passphrase for decrypting signing key
/// * `args` - Count command arguments
///
/// # Returns
/// Returns `Ok(())` on success.
pub async fn run(
    store_path: String,
    collection_name: String,
    passphrase: Option<String>,
    args: CountArgs,
) -> sentinel_dbms::Result<()> {
    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let collection = store
        .collection_with_config(&collection_name, Some(args.wal.to_overrides()))
        .await?;

    let count = collection.count().await?;
    info!(
        "Collection '{}' contains {} documents",
        collection_name, count
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_count_empty_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        // Create collection
        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let args = CountArgs {
            wal: crate::commands::WalArgs::default(),
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
    async fn test_count_collection_with_documents() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        // Create collection and insert documents
        let collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let docs = vec![
            ("doc1", serde_json::json!({"name": "Alice"})),
            ("doc2", serde_json::json!({"name": "Bob"})),
        ];
        collection.bulk_insert(docs).await.unwrap();

        let args = CountArgs {
            wal: crate::commands::WalArgs::default(),
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
}
