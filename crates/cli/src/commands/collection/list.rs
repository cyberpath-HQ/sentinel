use std::str::FromStr as _;

use clap::Args;
use sentinel_dbms::{
    futures::{pin_mut, StreamExt as _},
    VerificationMode,
    VerificationOptions,
};
use tracing::{error, info};

use crate::commands::WalArgs;

/// Arguments for collection list command.
#[derive(Args, Clone, Default)]
pub struct ListArgs {
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

impl ListArgs {
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

/// List all document IDs in a Sentinel collection.
///
/// This function streams all document IDs from the specified collection.
/// IDs are printed to stdout, one per line.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection` - Collection name
/// * `passphrase` - Optional passphrase for decrypting the signing key
/// * `args` - The parsed command-line arguments for collection list.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::collection::list::{run, ListArgs};
///
/// let args = ListArgs::default();
/// run("/tmp/my_store".to_string(), "users".to_string(), None, args).await?;
/// ```

pub async fn run(
    store_path: String,
    collection: String,
    passphrase: Option<String>,
    args: ListArgs,
) -> sentinel_dbms::Result<()> {
    info!(
        "Listing documents in collection '{}' in store {}",
        collection, store_path
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

    let verification_options = args.to_verification_options().map_err(|e| {
        sentinel_dbms::SentinelError::ConfigError {
            message: e,
        }
    })?;

    let stream = coll.all_with_verification(&verification_options);
    pin_mut!(stream);

    let mut count: usize = 0;
    // Process stream item by item to avoid loading all IDs into memory
    while let Some(item) = stream.next().await {
        match item {
            Ok(doc) => {
                #[allow(clippy::print_stdout, reason = "CLI output")]
                {
                    println!("{}", doc.id());
                }
                count = count.saturating_add(1);
            },
            Err(e) => {
                error!(
                    "Failed to list documents in collection '{}' in store {}: {}",
                    collection, store_path, e
                );
                return Err(e);
            },
        }
    }

    info!("Found {} documents in collection '{}'", count, collection);
    Ok(())
}
