use clap::Args;

/// Arguments for the get command.
#[derive(Args)]
pub struct GetArgs {
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

use std::io;

use tracing::{error, info, warn};

/// Retrieve a document from a Sentinel collection.
///
/// This function fetches the document with the specified ID from the given collection.
/// If the document exists, its JSON data is printed to stdout. If not found,
/// a warning is logged.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for get.
///
/// # Returns
/// Returns `Ok(())` on success, or an `io::Error` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::get::{run, GetArgs};
///
/// let args = GetArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     id:         "user1".to_string(),
/// };
/// run(args).await?;
/// ```
pub async fn run(args: GetArgs) -> io::Result<()> {
    let store_path = args.store_path;
    let collection = args.collection;
    let id = args.id;
    info!(
        "Getting document '{}' from collection '{}' in store {}",
        id, collection, store_path
    );
    let store = sentinel::Store::new(&store_path).await?;
    let coll = store.collection(&collection).await?;
    match coll.get(&id).await {
        Ok(Some(doc)) => {
            info!("Document '{}' retrieved successfully", id);
            println!("{}", serde_json::to_string_pretty(&doc.data)?);
            Ok(())
        },
        Ok(None) => {
            warn!("Document '{}' not found", id);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to get document '{}' from collection '{}' in store {}: {}",
                id, collection, store_path, e
            );
            Err(e)
        },
    }
}
