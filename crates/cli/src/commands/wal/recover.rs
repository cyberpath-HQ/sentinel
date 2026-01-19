//! WAL recovery command.

use clap::Args;
use tracing::{error, info, warn};

/// Arguments for the WAL recover command.
#[derive(Args)]
pub struct RecoverArgs;

/// Execute the WAL recovery operation.
pub async fn run(store_path: String, collection: Option<String>, _args: RecoverArgs) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::{CollectionWalOps, StoreWalOps};

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
