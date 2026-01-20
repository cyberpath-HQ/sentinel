use std::str::FromStr as _;

use clap::Args;
use sentinel_dbms::{CollectionWalConfig, VerificationMode, VerificationOptions, WalFailureMode};
use tracing::{error, info, warn};

use crate::commands::WalArgs;

/// Arguments for collection get command.
#[derive(Args, Clone, Default)]
pub struct GetArgs {
    /// Document ID
    #[arg(short, long)]
    pub id:               String,
    /// Verify document signature (default: true)
    #[arg(long, default_value = "true")]
    pub verify_signature: bool,
    /// Verify document hash (default: true)
    #[arg(long, default_value = "true")]
    pub verify_hash:      bool,
    /// Signature verification mode: strict, warn, or silent (default: strict)
    #[arg(long, default_value = "strict")]
    pub signature_mode:   String,
    /// How to handle documents with no signature: strict, warn, or silent (default: warn)
    #[arg(long, default_value = "warn")]
    pub empty_sig_mode:   String,
    /// Hash verification mode: strict, warn, or silent (default: strict)
    #[arg(long, default_value = "strict")]
    pub hash_mode:        String,
    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal:              WalArgs,
}

impl GetArgs {
    /// Convert CLI arguments to verification options.
    fn to_verification_options(&self) -> Result<VerificationOptions, String> {
        let signature_verification_mode = VerificationMode::from_str(&self.signature_mode).map_err(|_| {
            format!(
                "Invalid signature verification mode: {}",
                self.signature_mode
            )
        })?;

        let empty_signature_mode = VerificationMode::from_str(&self.empty_sig_mode)
            .map_err(|_| format!("Invalid empty signature mode: {}", self.empty_sig_mode))?;

        let hash_verification_mode = VerificationMode::from_str(&self.hash_mode)
            .map_err(|_| format!("Invalid hash verification mode: {}", self.hash_mode))?;

        Ok(VerificationOptions {
            verify_signature: self.verify_signature,
            verify_hash: self.verify_hash,
            signature_verification_mode,
            empty_signature_mode,
            hash_verification_mode,
        })
    }
}

/// Retrieve a document from a Sentinel collection.
///
/// This function fetches document with the specified ID from the given collection.
/// If the document exists, its JSON data is printed to stdout. If not found,
/// a warning is logged.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection` - Collection name
/// * `passphrase` - Optional passphrase for decrypting the signing key
/// * `args` - The parsed command-line arguments for collection get.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::collection::get::{run, GetArgs};
///
/// let args = GetArgs {
///     id:               "user1".to_string(),
///     verify_signature: true,
///     verify_hash:      true,
///     signature_mode:   "strict".to_string(),
///     hash_mode:        "strict".to_string(),
///     wal:              WalArgs::default(),
/// };
/// run("/tmp/my_store".to_string(), "users".to_string(), None, args).await?;
/// ```

/// Build CollectionWalConfig from CLI arguments
fn build_collection_wal_config(args: &GetArgs, global_wal: &WalArgs) -> Option<CollectionWalConfig> {
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
    args: GetArgs,
) -> sentinel_dbms::Result<()> {
    let id = &args.id;
    info!(
        "Getting document '{}' from collection '{}' in store {}",
        id, collection, store_path
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

    let verification_options = args.to_verification_options().map_err(|e| {
        sentinel_dbms::SentinelError::ConfigError {
            message: e,
        }
    })?;

    match coll.get_with_verification(id, &verification_options).await {
        Ok(Some(doc)) => {
            info!("Document '{}' retrieved successfully", id);
            match serde_json::to_string_pretty(doc.data()) {
                Ok(json) => {
                    #[allow(clippy::print_stdout, reason = "CLI output")]
                    {
                        println!("{}", json);
                    }
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to serialize document to JSON: {}", e);
                    Err(sentinel_dbms::SentinelError::Json {
                        source: e,
                    })
                },
            }
        },
        Ok(None) => {
            warn!("Document '{}' not found in collection '{}'", id, collection);
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
