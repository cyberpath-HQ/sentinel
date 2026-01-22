use clap::Args;
use serde_json::Value;
use tracing::{error, info};

use crate::commands::WalArgs;

/// Arguments for the collection update command.
#[derive(Args, Clone)]
pub struct UpdateArgs {
    /// Document ID
    #[arg(short, long)]
    pub id:   String,
    /// JSON data (as string)
    #[arg(short, long)]
    pub data: String,
    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal:  WalArgs,
}

/// Update an existing document in a Sentinel collection.
///
/// This function replaces the entire document with the specified ID with new JSON data.
/// It validates the JSON format and handles any errors during the update process.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection` - Collection name
/// * `passphrase` - Optional passphrase for decrypting the signing key
/// * `args` - The parsed command-line arguments for collection update.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::collection::update::{run, UpdateArgs};
///
/// let args = UpdateArgs {
///     id:   String::from("user1"),
///     data: r#"{"name": "Bob"}"#.to_string(),
///     wal:  WalArgs::default(),
/// };
/// run(
///     String::from("/tmp/my_store"),
///     String::from("users"),
///     None,
///     args,
/// )
/// .await?;
/// ```

pub async fn run(
    store_path: String,
    collection: String,
    passphrase: Option<String>,
    args: UpdateArgs,
) -> sentinel_dbms::Result<()> {
    info!(
        "Updating document '{}' in collection '{}' in store {}",
        args.id, collection, store_path
    );
    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let coll = store
        .collection_with_config(&collection, Some(args.wal.to_overrides()))
        .await?;
    let value: Value = match serde_json::from_str(&args.data) {
        Ok(v) => v,
        Err(e) => {
            error!("Invalid JSON data: {}", e);
            return Err(sentinel_dbms::SentinelError::Json {
                source: e,
            });
        },
    };
    match coll.update(&args.id, value).await {
        Ok(_) => {
            info!("Document '{}' updated successfully", args.id);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to update document '{}' in collection '{}' in store {}: {}",
                args.id, collection, store_path, e
            );
            Err(e)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_update_existing_document() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");
        let collection_name = "test_collection";

        // Create store and collection
        let store = sentinel_dbms::Store::new_with_config(
            &store_path,
            None,
            sentinel_dbms::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config(collection_name, None).await.unwrap();

        // Insert a document first
        collection.insert("doc1", serde_json::json!({"name": "Alice", "age": 30})).await.unwrap();

        // Allow event processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Update the document
        let args = UpdateArgs {
            id: "doc1".to_string(),
            data: r#"{"name": "Alice", "age": 31}"#.to_string(),
            wal: WalArgs::default(),
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
    async fn test_update_nonexistent_document() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");
        let collection_name = "test_collection";

        // Create store and collection
        let store = sentinel_dbms::Store::new_with_config(
            &store_path,
            None,
            sentinel_dbms::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let _collection = store.collection_with_config(collection_name, None).await.unwrap();

        // Try to update a non-existent document
        let args = UpdateArgs {
            id: "nonexistent".to_string(),
            data: r#"{"name": "Bob"}"#.to_string(),
            wal: WalArgs::default(),
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
    async fn test_update_invalid_json() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");
        let collection_name = "test_collection";

        // Create store and collection
        let store = sentinel_dbms::Store::new_with_config(
            &store_path,
            None,
            sentinel_dbms::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config(collection_name, None).await.unwrap();

        // Insert a document first
        collection.insert("doc1", serde_json::json!({"name": "Alice"})).await.unwrap();

        // Allow event processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Try to update with invalid JSON
        let args = UpdateArgs {
            id: "doc1".to_string(),
            data: r#"{"name": "Alice", "age": }"#.to_string(), // Invalid JSON
            wal: WalArgs::default(),
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