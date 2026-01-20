use clap::Args;
use tracing::{error, info};

use crate::commands::WalArgs;

/// Arguments for the collection delete command.
#[derive(Args, Clone)]
pub struct DeleteArgs {
    /// Document ID
    #[arg(short, long)]
    pub id:  String,
    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal: WalArgs,
}

/// Delete a document from a Sentinel collection.
///
/// This function removes the document with the specified ID from the given collection.
/// It handles any errors that may occur during the deletion process.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection` - Collection name
/// * `passphrase` - Optional passphrase for decrypting the signing key
/// * `args` - The parsed command-line arguments for collection delete.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::collection::delete::{run, DeleteArgs};
///
/// let args = DeleteArgs {
///     id:  "user1".to_string(),
///     wal: WalArgs::default(),
/// };
/// run("/tmp/my_store".to_string(), "users".to_string(), None, args).await?;
/// ```

pub async fn run(
    store_path: String,
    collection: String,
    passphrase: Option<String>,
    args: DeleteArgs,
) -> sentinel_dbms::Result<()> {
    let id = args.id;
    info!(
        "Deleting document '{}' from collection '{}' in store {}",
        id, collection, store_path
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
    match coll.delete(&id).await {
        Ok(_) => {
            info!("Document '{}' deleted successfully", id);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to delete document '{}' from collection '{}' in store {}: {}",
                id, collection, store_path, e
            );
            Err(e)
        },
    }
}
