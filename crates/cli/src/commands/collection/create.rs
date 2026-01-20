use clap::Args;
use tracing::{error, info};
use sentinel_dbms::{CollectionWalConfig, CompressionAlgorithm, WalFailureMode, WalFormat};

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

/// Build CollectionWalConfig from CLI arguments
fn build_collection_wal_config(args: &CreateArgs, global_wal: &WalArgs) -> Option<CollectionWalConfig> {
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
    args: CreateArgs,
) -> sentinel_dbms::Result<()> {
    let wal_config = build_collection_wal_config(&args, &args.wal);
    info!(
        "Creating collection '{}' in store {}",
        collection, store_path
    );
    // For create_collection, we need to open the store with its existing config
    // and then create the collection with the specified WAL config
    let store = sentinel_dbms::Store::new(&store_path, passphrase.as_deref()).await?;
    match store.collection_with_config(&collection, wal_config).await {
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
