use clap::Args;
use sentinel_dbms::{CollectionWalConfig, WalFailureMode};
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

/// Build CollectionWalConfig from CLI arguments
fn build_collection_wal_config(args: &DeleteArgs, global_wal: &WalArgs) -> Option<CollectionWalConfig> {
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
    args: DeleteArgs,
) -> sentinel_dbms::Result<()> {
    let wal_config = build_collection_wal_config(&args, &args.wal);
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
        .collection_with_config(&collection, wal_config)
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
