use clap::Args;
use tracing::{error, info};
use sentinel_dbms::CollectionWalConfigOverrides;

use crate::commands::WalArgs;

/// Arguments for the collection create command.
#[derive(Args, Clone, Default)]
pub struct CreateArgs {
    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal: WalArgs,
}

/// Create a new collection within an existing Sentinel store.
///
/// This function creates a logical grouping for documents within the specified store.
/// It validates that the store exists and handles any errors during collection creation.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection` - Name of the collection to create
/// * `passphrase` - Optional passphrase for decrypting the signing key
/// * `args` - The parsed command-line arguments for collection create.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::collection::create::{run, CreateArgs};
///
/// let args = CreateArgs::default();
/// run("/tmp/my_store".to_string(), "users".to_string(), None, args).await?;
/// ```

/// Build CollectionWalConfigOverrides from CLI arguments
fn build_collection_wal_config_overrides(
    args: &CreateArgs,
    global_wal: &WalArgs,
) -> Option<CollectionWalConfigOverrides> {
    // Only build overrides if any WAL options are provided
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
        global_wal.wal_enable_recovery.is_some() ||
        args.wal.wal_persist_overrides ||
        global_wal.wal_persist_overrides
    {
        Some(CollectionWalConfigOverrides {
            write_mode:            args.wal.wal_write_mode.or(global_wal.wal_write_mode),
            verification_mode:     args.wal.wal_verify_mode.or(global_wal.wal_verify_mode),
            auto_verify:           args.wal.wal_auto_verify.or(global_wal.wal_auto_verify),
            enable_recovery:       args
                .wal
                .wal_enable_recovery
                .or(global_wal.wal_enable_recovery),
            max_wal_size_bytes:    args
                .wal
                .wal_max_file_size
                .or(global_wal.wal_max_file_size)
                .map(Some),
            compression_algorithm: args
                .wal
                .wal_compression
                .or(global_wal.wal_compression)
                .map(Some),
            max_records_per_file:  args
                .wal
                .wal_max_records
                .or(global_wal.wal_max_records)
                .map(Some),
            format:                args.wal.wal_format.or(global_wal.wal_format),
            persist_overrides:     args.wal.wal_persist_overrides || global_wal.wal_persist_overrides,
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
    args: CreateArgs,
) -> sentinel_dbms::Result<()> {
    let wal_overrides = build_collection_wal_config_overrides(&args, &args.wal);
    info!(
        "Creating collection '{}' in store {}",
        collection, store_path
    );
    // For create_collection, we need to open the store with its existing config
    // and then create the collection with the specified WAL config
    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    match store
        .collection_with_config(&collection, wal_overrides)
        .await
    {
        Ok(_) => {
            info!("Collection '{}' created successfully", collection);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to create collection '{}' in store {}: {}",
                collection, store_path, e
            );
            Err(e)
        },
    }
}
