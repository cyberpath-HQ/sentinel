//! WAL stats command.

use clap::Args;
use tracing::info;

/// Arguments for the WAL stats command.
#[derive(Args)]
pub struct StatsArgs;

/// Execute the WAL stats operation.
#[allow(clippy::arithmetic_side_effects, reason = "Safe arithmetic for calculating WAL statistics and averages")]
pub async fn run(store_path: String, collection: Option<String>, _args: StatsArgs) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::CollectionWalOps as _;

    let store =
        sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default()).await?;

    if let Some(collection_name) = collection {
        let collection = store.collection_with_config(&collection_name, None).await?;

        let size = collection.wal_size().await?;
        let count = collection.wal_entries_count().await?;

        info!("WAL Statistics for collection '{}':", collection_name);
        tracing::info!(
            "  Size: {} bytes ({:.2} MB)",
            size,
            size as f64 / (1024.0 * 1024.0)
        );
        tracing::info!("  Entries: {}", count);
        
        tracing::info!(
            "  Average entry size: {} bytes",
            if count > 0 { size.checked_div(count as u64).unwrap_or(0) } else { 0 }
        );
    }
    else {
        info!("WAL Statistics for all collections:");

        let collections = store.list_collections().await?;
        let mut total_size = 0u64;
        let mut total_entries = 0usize;

        for collection_name in collections {
            if let Ok(collection) = store.collection_with_config(&collection_name, None).await
                && let (Ok(size), Ok(count)) = (
                    collection.wal_size().await,
                    collection.wal_entries_count().await,
                ) {
                    total_size += size;
                    total_entries += count;

                    tracing::info!("  {}: {} bytes,  {} entries", collection_name, size, count);
                }
        }

        tracing::info!(
            "  Total: {} bytes ({:.2} MB), {} entries",
            total_size,
            total_size as f64 / (1024.0 * 1024.0),
            total_entries
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use sentinel_dbms::StoreWalConfig;
    use tempfile::TempDir;
    use tokio::time::{sleep, Duration};

    use super::*;

    #[tokio::test]
    async fn test_wal_stats_specific_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, StoreWalConfig::default())
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

        // Run stats command
        let args = StatsArgs;
        let result = run(
            store_path.clone(),
            Some("test_collection".to_string()),
            args,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wal_stats_all_collections() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create store and collections
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, StoreWalConfig::default())
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

        // Run stats command for all collections
        let args = StatsArgs;
        let result = run(store_path.clone(), None, args).await;

        assert!(result.is_ok());
    }
}
