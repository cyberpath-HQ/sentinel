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
