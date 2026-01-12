use clap::Args;

/// Arguments for the insert command.
#[derive(Args)]
pub struct InsertArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path: String,
    /// Collection name
    #[arg(short, long)]
    pub collection: String,
    /// Document ID
    #[arg(short, long)]
    pub id: String,
    /// JSON data (as string)
    #[arg(short, long)]
    pub data: String,
}

use serde_json::Value;
use std::io;
use tracing::{error, info};

/// Insert a new document into a Sentinel collection.
///
/// This function parses the provided JSON data and inserts it as a new document
/// with the specified ID into the given collection. It validates the JSON format
/// and handles any errors during the insertion process.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for insert.
///
/// # Returns
/// Returns `Ok(())` on success, or an `io::Error` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::insert::{InsertArgs, run};
///
/// let args = InsertArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     id: "user1".to_string(),
///     data: r#"{"name": "Alice"}"#.to_string(),
/// };
/// run(args).await?;
/// ```
pub async fn run(args: InsertArgs) -> io::Result<()> {
    let store_path = args.store_path;
    let collection = args.collection;
    let id = args.id;
    let data = args.data;
    info!("Inserting document '{}' into collection '{}' in store {}", id, collection, store_path);
    let store = sentinel::Store::new(&store_path).await?;
    let coll = store.collection(&collection).await?;
    let value: Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(e) => {
            error!("Invalid JSON data: {}", e);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
        }
    };
    match coll.insert(&id, value).await {
        Ok(_) => {
            info!("Document '{}' inserted successfully", id);
            Ok(())
        }
        Err(e) => {
            error!("Failed to insert document '{}' into collection '{}' in store {}: {}", id, collection, store_path, e);
            Err(e)
        }
    }
}