use clap::Args;

/// Arguments for collection info command.
#[derive(Args)]
pub struct InfoArgs {
    /// Output format: table (default) or json
    #[arg(long, default_value = "table")]
    pub format: String,
}

/// Execute collection info command.
///
/// Displays metadata and statistics for the specified collection.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection_name` - Name of the collection
/// * `passphrase` - Optional passphrase for decrypting signing key
/// * `args` - Info command arguments
///
/// # Returns
/// Returns `Ok(())` on success.
pub async fn run(
    store_path: String,
    collection_name: String,
    passphrase: Option<String>,
    args: InfoArgs,
) -> sentinel_dbms::Result<()> {
    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let collection = store.collection_with_config(&collection_name, None).await?;

    match args.format.as_str() {
        "table" => {
            println!("Collection Information");
            println!("====================");
            println!("Name:              {}", collection.name());
            println!(
                "Created At:        {}",
                collection.created_at().format("%Y-%m-%d %H:%M:%S UTC")
            );
            println!(
                "Updated At:        {}",
                collection.updated_at().format("%Y-%m-%d %H:%M:%S UTC")
            );

            if let Some(checkpoint) = collection.last_checkpoint_at() {
                println!(
                    "Last Checkpoint:   {}",
                    checkpoint.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
            else {
                println!("Last Checkpoint:   Never");
            }

            println!("Total Documents:   {}", collection.total_documents());
            println!(
                "Total Size:        {} bytes ({:.2} MB)",
                collection.total_size_bytes(),
                collection.total_size_bytes() as f64 / 1_000_000.0
            );
        },
        _ => {
            return Err(sentinel_dbms::SentinelError::Internal {
                message: format!("Invalid format: {}. Use 'table'", args.format),
            });
        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn test_info_command_with_empty_collection() {
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

        // Run info command
        let args = InfoArgs {
            format: "table".to_string(),
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
    async fn test_info_command_with_populated_collection() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");
        let collection_name = "test_collection";

        // Create store and collection with some data
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        let collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        // Insert some documents
        collection
            .insert("doc1", json!({"name": "Alice", "age": 30}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"name": "Bob", "age": 25}))
            .await
            .unwrap();

        // Create a checkpoint
        collection.checkpoint_wal().await.unwrap();

        // Allow event processor to update counters
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Run info command
        let args = InfoArgs {
            format: "table".to_string(),
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
    async fn test_info_command_invalid_format() {
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

        // Run info command with invalid format
        let args = InfoArgs {
            format: "invalid".to_string(),
        };
        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid format"));
    }

    #[tokio::test]
    async fn test_info_command_invalid_store_path() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("invalid_store");
        std::fs::write(&store_path, "not a directory").unwrap();
        let collection_name = "test_collection";

        let args = InfoArgs {
            format: "table".to_string(),
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
}
