use clap::Args;
use tracing::{error, info};

/// Arguments for the delete command.
#[derive(Args)]
pub struct DeleteArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path: String,
    /// Collection name
    #[arg(short, long)]
    pub collection: String,
    /// Document ID
    #[arg(short, long)]
    pub id:         String,
}


/// Delete a document from a Sentinel collection.
///
/// This function removes the document with the specified ID from the given collection.
/// It handles any errors that may occur during the deletion process.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for delete.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::delete::{run, DeleteArgs};
///
/// let args = DeleteArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     id:         "user1".to_string(),
/// };
/// run(args).await?;
/// ```
pub async fn run(args: DeleteArgs) -> sentinel::Result<()> {
    let store_path = args.store_path;
    let collection = args.collection;
    let id = args.id;
    info!(
        "Deleting document '{}' from collection '{}' in store {}",
        id, collection, store_path
    );
    let store = sentinel::Store::new(&store_path).await?;
    let coll = store.collection(&collection).await?;
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
