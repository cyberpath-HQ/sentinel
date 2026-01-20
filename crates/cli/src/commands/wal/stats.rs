//! WAL stats command.

use clap::Args;
use tracing::info;

/// Arguments for the WAL stats command.
#[derive(Args)]
pub struct StatsArgs;

/// Execute the WAL stats operation.
pub async fn run(store_path: String, collection: Option<String>, _args: StatsArgs) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::CollectionWalOps;

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
            if count > 0 { size / count as u64 } else { 0 }
        );
    }
    else {
        info!("WAL Statistics for all collections:");

        let collections = store.list_collections().await?;
        let mut total_size = 0u64;
        let mut total_entries = 0usize;

        for collection_name in collections {
            if let Ok(collection) = store.collection_with_config(&collection_name, None).await {
                if let (Ok(size), Ok(count)) = (
                    collection.wal_size().await,
                    collection.wal_entries_count().await,
                ) {
                    total_size += size;
                    total_entries += count;

                    tracing::info!("  {}: {} bytes,  {} entries", collection_name, size, count);
                }
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
