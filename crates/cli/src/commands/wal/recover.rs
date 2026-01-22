//! WAL recovery command.

use clap::Args;
use tracing::{error, info, warn};

/// Arguments for the WAL recover command.
#[derive(Args)]
pub struct RecoverArgs;

/// Execute the WAL recovery operation.
pub async fn run(store_path: String, collection: Option<String>, _args: RecoverArgs) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::{CollectionWalOps as _, StoreWalOps};

    let store =
        sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default()).await?;

    if let Some(collection_name) = collection {
        let collection = store.collection_with_config(&collection_name, None).await?;
        info!(
            "Recovering data from WAL for collection '{}'...",
            collection_name
        );

        let result = collection.recover_from_wal().await?;
        info!("Recovery completed for collection '{}'", collection_name);
        info!("  Operations recovered: {}", result.recovered_operations);
        info!("  Operations skipped: {}", result.skipped_operations);

        if result.failed_operations > 0 {
            warn!("  Operations failed: {}", result.failed_operations);
            for failure in &result.failures {
                error!("    - {:?}", failure);
            }
        }
    }
    else {
        info!("Recovering data from WAL for all collections...");
        let recovery_stats = store.recover_all_collections().await?;

        let total_operations: usize = recovery_stats.values().sum();
        info!(
            "Recovery completed for {} collections",
            recovery_stats.len()
        );
        info!("  Total operations recovered: {}", total_operations);

        for (collection_name, count) in recovery_stats {
            info!("  Collection '{}': {} operations", collection_name, count);
        }
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
    async fn test_recover_from_wal_specific_collection() {
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
            .insert("doc1", serde_json::json!({"name": "test1"}))
            .await
            .unwrap();
        collection
            .insert("doc2", serde_json::json!({"name": "test2"}))
            .await
            .unwrap();

        // Wait for events to be processed
        sleep(Duration::from_millis(100)).await;

        // Run recover command
        let args = RecoverArgs;
        let result = run(store_path.clone(), Some("test_collection".to_string()), args).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_recover_from_wal_all_collections() {
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

        // Run recover command for all collections
        let args = RecoverArgs;
        let result = run(store_path.clone(), None, args).await;

        assert!(result.is_ok());
    }
}
