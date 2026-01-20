use clap::Args;
use serde_json::Value;
use tracing::{error, info};

use crate::commands::WalArgs;

/// Arguments for the collection update command.
#[derive(Args, Clone)]
pub struct UpdateArgs {
    /// Document ID
    #[arg(short, long)]
    pub id:   String,
    /// JSON data (as string)
    #[arg(short, long)]
    pub data: String,
    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal:  WalArgs,
}

/// Update an existing document in a Sentinel collection.
///
/// This function replaces the entire document with the specified ID with new JSON data.
/// It validates the JSON format and handles any errors during the update process.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection` - Collection name
/// * `passphrase` - Optional passphrase for decrypting the signing key
/// * `args` - The parsed command-line arguments for collection update.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::collection::update::{run, UpdateArgs};
///
/// let args = UpdateArgs {
///     id:   "user1".to_string(),
///     data: r#"{"name": "Bob"}"#.to_string(),
///     wal:  WalArgs::default(),
/// };
/// run("/tmp/my_store".to_string(), "users".to_string(), None, args).await?;
/// ```

pub async fn run(
    store_path: String,
    collection: String,
    passphrase: Option<String>,
    args: UpdateArgs,
) -> sentinel_dbms::Result<()> {
    info!(
        "Updating document '{}' in collection '{}' in store {}",
        args.id, collection, store_path
    );
    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let coll = store.collection_with_config(&collection, None).await?;
    let value: Value = match serde_json::from_str(&args.data) {
        Ok(v) => v,
        Err(e) => {
            error!("Invalid JSON data: {}", e);
            return Err(sentinel_dbms::SentinelError::Json {
                source: e,
            });
        },
    };
    match coll.update(&args.id, value).await {
        Ok(_) => {
            info!("Document '{}' updated successfully", args.id);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to update document '{}' in collection '{}' in store {}: {}",
                args.id, collection, store_path, e
            );
            Err(e)
        },
    }
}
