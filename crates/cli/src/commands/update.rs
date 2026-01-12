use clap::Args;

/// Arguments for the update command.
#[derive(Args)]
pub struct UpdateArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path: String,
    /// Collection name
    #[arg(short, long)]
    pub collection: String,
    /// Document ID
    #[arg(short, long)]
    pub id:         String,
    /// JSON data (as string)
    #[arg(short, long)]
    pub data:       String,
}

use std::io;

use serde_json::Value;
use tracing::{error, info};

/// Update an existing document in a Sentinel collection.
///
/// This function replaces the entire document with the specified ID with new JSON data.
/// It validates the JSON format and handles any errors during the update process.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for update.
///
/// # Returns
/// Returns `Ok(())` on success, or an `io::Error` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::update::{run, UpdateArgs};
///
/// let args = UpdateArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     id:         "user1".to_string(),
///     data:       r#"{"name": "Bob"}"#.to_string(),
/// };
/// run(args).await?;
/// ```
pub async fn run(args: UpdateArgs) -> io::Result<()> {
    let store_path = args.store_path;
    let collection = args.collection;
    let id = args.id;
    let data = args.data;
    info!(
        "Updating document '{}' in collection '{}' in store {}",
        id, collection, store_path
    );
    let store = sentinel::Store::new(&store_path).await?;
    let coll = store.collection(&collection).await?;
    let value: Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(e) => {
            error!("Invalid JSON data: {}", e);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
        },
    };
    match coll.update(&id, value).await {
        Ok(_) => {
            info!("Document '{}' updated successfully", id);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to update document '{}' in collection '{}' in store {}: {}",
                id, collection, store_path, e
            );
            Err(e)
        },
    }
}
