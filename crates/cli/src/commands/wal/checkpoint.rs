//! WAL checkpoint command.

use clap::Args;
use tracing::info;

/// Arguments for the WAL checkpoint command.
#[derive(Args)]
pub struct CheckpointArgs;

/// Execute the WAL checkpoint operation.
pub async fn run(store_path: String, collection: Option<String>, _args: CheckpointArgs) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::{CollectionWalOps as _, StoreWalOps};

    let store =
        sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default()).await?;

    if let Some(collection_name) = collection {
        let collection = store.collection_with_config(&collection_name, None).await?;
        info!(
            "Creating WAL recovery point for collection '{}'...",
            collection_name
        );
        collection.checkpoint_wal().await?;
        info!(
            "WAL recovery point created for collection '{}'",
            collection_name
        );
    }
    else {
        info!("Creating WAL recovery points for all collections...");
        store.checkpoint_all_collections().await?;
        info!("WAL recovery points created for all collections");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentinel_dbms::StoreWalConfig;
    use tempfile::TempDir;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_checkpoint_specific_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create store and collection
        let store = sentinel_dbms::Store::new_with_config(
            &store_path,
            None,
            StoreWalConfig::default(),
        )
        .await
        .unwrap();

        let collection = store
            .collection_with_config("test_collection", None)
            .await
            .unwrap();

        // Insert some data to create WAL entries
        collection
            .insert("doc1", serde_json::json!({"name": "test"}))
            .await
            .unwrap();

        // Wait for events to be processed
        sleep(Duration::from_millis(100)).await;

        // Run checkpoint command
        let args = CheckpointArgs;
        let result = run(store_path.clone(), Some("test_collection".to_string()), args).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_checkpoint_all_collections() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create store and collections
        let store = sentinel_dbms::Store::new_with_config(
            &store_path,
            None,
            StoreWalConfig::default(),
        )
        .await
        .unwrap();

        let collection1 = store
            .collection_with_config("collection1", None)
            .await
            .unwrap();
        let collection2 = store
            .collection_with_config("collection2", None)
            .await
            .unwrap();

        // Insert some data to create WAL entries
        collection1
            .insert("doc1", serde_json::json!({"name": "test1"}))
            .await
            .unwrap();
        collection2
            .insert("doc2", serde_json::json!({"name": "test2"}))
            .await
            .unwrap();

        // Wait for events to be processed
        sleep(Duration::from_millis(100)).await;

        // Run checkpoint command for all collections
        let args = CheckpointArgs;
        let result = run(store_path.clone(), None, args).await;

        assert!(result.is_ok());
    }
}
