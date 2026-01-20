use clap::Args;
use serde_json::Value;
use tracing::{error, info};

use crate::commands::WalArgs;

/// Arguments for the collection upsert command.
#[derive(Args, Clone)]
pub struct UpsertArgs {
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

/// Insert or update a document in a Sentinel collection.
///
/// This function creates a new document if it doesn't exist, or updates it if it does.
/// It returns whether the operation was an insert (true) or update (false).
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection` - Collection name
/// * `passphrase` - Optional passphrase for decrypting the signing key
/// * `args` - The parsed command-line arguments for collection upsert.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::collection::upsert::{run, UpsertArgs};
///
/// let args = UpsertArgs {
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
    args: UpsertArgs,
) -> sentinel_dbms::Result<()> {
    info!(
        "Upserting document '{}' in collection '{}' in store {}",
        args.id, collection, store_path
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
    let value: Value = match serde_json::from_str(&args.data) {
        Ok(v) => v,
        Err(e) => {
            error!("Invalid JSON data: {}", e);
            return Err(sentinel_dbms::SentinelError::Json {
                source: e,
            });
        },
    };
    match coll.upsert(&args.id, value).await {
        Ok(was_insert) => {
            if was_insert {
                info!("Document '{}' inserted successfully", args.id);
            }
            else {
                info!("Document '{}' updated successfully", args.id);
            }
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to upsert document '{}' in collection '{}' in store {}: {}",
                args.id, collection, store_path, e
            );
            Err(e)
        },
    }
}
