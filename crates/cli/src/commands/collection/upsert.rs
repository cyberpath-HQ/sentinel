use clap::Args;
use serde_json::Value;
use tracing::{error, info};
use sentinel_dbms::{CollectionWalConfig, WalFailureMode};

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

/// Build CollectionWalConfig from CLI arguments
fn build_collection_wal_config(args: &UpsertArgs, global_wal: &WalArgs) -> Option<CollectionWalConfig> {
    // Only build config if any WAL options are provided
    if args.wal.wal_max_file_size.is_some() ||
        args.wal.wal_format.is_some() ||
        args.wal.wal_compression.is_some() ||
        args.wal.wal_max_records.is_some() ||
        args.wal.wal_write_mode.is_some() ||
        args.wal.wal_verify_mode.is_some() ||
        args.wal.wal_auto_verify.is_some() ||
        args.wal.wal_enable_recovery.is_some() ||
        global_wal.wal_max_file_size.is_some() ||
        global_wal.wal_format.is_some() ||
        global_wal.wal_compression.is_some() ||
        global_wal.wal_max_records.is_some() ||
        global_wal.wal_write_mode.is_some() ||
        global_wal.wal_verify_mode.is_some() ||
        global_wal.wal_auto_verify.is_some() ||
        global_wal.wal_enable_recovery.is_some()
    {
        Some(CollectionWalConfig {
            write_mode:            args
                .wal
                .wal_write_mode
                .or(global_wal.wal_write_mode)
                .unwrap_or(WalFailureMode::Strict),
            verification_mode:     args
                .wal
                .wal_verify_mode
                .or(global_wal.wal_verify_mode)
                .unwrap_or(WalFailureMode::Warn),
            auto_verify:           args
                .wal
                .wal_auto_verify
                .or(global_wal.wal_auto_verify)
                .unwrap_or(false),
            enable_recovery:       args
                .wal
                .wal_enable_recovery
                .or(global_wal.wal_enable_recovery)
                .unwrap_or(true),
            max_wal_size_bytes:    args.wal.wal_max_file_size.or(global_wal.wal_max_file_size),
            compression_algorithm: args.wal.wal_compression.or(global_wal.wal_compression),
            max_records_per_file:  args.wal.wal_max_records.or(global_wal.wal_max_records),
            format:                args
                .wal
                .wal_format
                .or(global_wal.wal_format)
                .unwrap_or_default(),
        })
    }
    else {
        None
    }
}

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
    let wal_config = build_collection_wal_config(&args, &args.wal);
    let coll = store
        .collection_with_config(&collection, wal_config)
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
