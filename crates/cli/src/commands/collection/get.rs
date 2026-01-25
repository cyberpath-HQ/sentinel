use std::str::FromStr as _;

use clap::Args;
use sentinel_dbms::{VerificationMode, VerificationOptions};
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
///     id:               String::from("user1"),
///     verify_signature: true,
///     verify_hash:      true,
///     signature_mode:   String::from("strict"),
///     hash_mode:        String::from("strict"),
///     wal:              WalArgs::default(),
/// };
/// run(
///     String::from("/tmp/my_store"),
///     String::from("users"),
///     None,
///     args,
/// )
/// .await?;
/// ```

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
    let coll = store
        .collection_with_config(&collection, Some(args.wal.to_overrides()))
        .await?;

    let verification_options = args.to_verification_options().map_err(|e| {
        sentinel_dbms::SentinelError::ConfigError {
            message: e,
        }
    })?;

    match coll.get_with_verification(id, &verification_options).await {
        Ok(Some(doc)) => {
            info!("Document '{}' retrieved successfully", id);
            let json = serde_json::to_string_pretty(doc.data()).unwrap();
            #[allow(clippy::print_stdout, reason = "CLI output")]
            {
                println!("{}", json);
            }
            Ok(())
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

#[cfg(test)]
mod tests {
    use sentinel_dbms::VerificationMode;
    use tempfile::TempDir;

    use super::*;
    use crate::commands::WalArgs;

    #[test]
    fn test_valid_signature_modes() {
        let args = GetArgs {
            id:               "test".to_string(),
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
            wal:              WalArgs::default(),
        };

        let result = args.to_verification_options();
        assert!(result.is_ok());
        let opts = result.unwrap();
        assert_eq!(opts.signature_verification_mode, VerificationMode::Strict);

        let args = GetArgs {
            signature_mode: "warn".to_string(),
            ..args
        };
        let result = args.to_verification_options();
        assert!(result.is_ok());
        let opts = result.unwrap();
        assert_eq!(opts.signature_verification_mode, VerificationMode::Warn);

        let args = GetArgs {
            signature_mode: "silent".to_string(),
            ..args
        };
        let result = args.to_verification_options();
        assert!(result.is_ok());
        let opts = result.unwrap();
        assert_eq!(opts.signature_verification_mode, VerificationMode::Silent);
    }

    #[test]
    fn test_valid_hash_modes() {
        let args = GetArgs {
            id:               "test".to_string(),
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
            wal:              WalArgs::default(),
        };

        let result = args.to_verification_options();
        assert!(result.is_ok());
        let opts = result.unwrap();
        assert_eq!(opts.hash_verification_mode, VerificationMode::Strict);

        let args = GetArgs {
            hash_mode: "warn".to_string(),
            ..args
        };
        let result = args.to_verification_options();
        assert!(result.is_ok());
        let opts = result.unwrap();
        assert_eq!(opts.hash_verification_mode, VerificationMode::Warn);

        let args = GetArgs {
            hash_mode: "silent".to_string(),
            ..args
        };
        let result = args.to_verification_options();
        assert!(result.is_ok());
        let opts = result.unwrap();
        assert_eq!(opts.hash_verification_mode, VerificationMode::Silent);
    }

    #[test]
    fn test_valid_empty_signature_modes() {
        let args = GetArgs {
            id:               "test".to_string(),
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "strict".to_string(),
            hash_mode:        "strict".to_string(),
            wal:              WalArgs::default(),
        };

        let result = args.to_verification_options();
        assert!(result.is_ok());
        let opts = result.unwrap();
        assert_eq!(opts.empty_signature_mode, VerificationMode::Strict);

        let args = GetArgs {
            empty_sig_mode: "warn".to_string(),
            ..args
        };
        let result = args.to_verification_options();
        assert!(result.is_ok());
        let opts = result.unwrap();
        assert_eq!(opts.empty_signature_mode, VerificationMode::Warn);

        let args = GetArgs {
            empty_sig_mode: "silent".to_string(),
            ..args
        };
        let result = args.to_verification_options();
        assert!(result.is_ok());
        let opts = result.unwrap();
        assert_eq!(opts.empty_signature_mode, VerificationMode::Silent);
    }

    #[test]
    fn test_invalid_signature_mode_returns_error() {
        let args = GetArgs {
            id:               "test".to_string(),
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "invalid_mode".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
            wal:              WalArgs::default(),
        };

        let result = args.to_verification_options();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Invalid signature verification mode: invalid_mode"));
    }

    #[test]
    fn test_invalid_hash_mode_returns_error() {
        let args = GetArgs {
            id:               "test".to_string(),
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "invalid_hash".to_string(),
            wal:              WalArgs::default(),
        };

        let result = args.to_verification_options();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Invalid hash verification mode: invalid_hash"));
    }

    #[test]
    fn test_invalid_empty_signature_mode_returns_error() {
        let args = GetArgs {
            id:               "test".to_string(),
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "invalid_empty".to_string(),
            hash_mode:        "strict".to_string(),
            wal:              WalArgs::default(),
        };

        let result = args.to_verification_options();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Invalid empty signature mode: invalid_empty"));
    }

    #[test]
    fn test_combinations_of_valid_options_produce_correct_verification_options() {
        // Test combination of different modes
        let args = GetArgs {
            id:               "test".to_string(),
            verify_signature: false,
            verify_hash:      true,
            signature_mode:   "warn".to_string(),
            empty_sig_mode:   "silent".to_string(),
            hash_mode:        "warn".to_string(),
            wal:              WalArgs::default(),
        };

        let result = args.to_verification_options();
        assert!(result.is_ok());
        let opts = result.unwrap();

        assert_eq!(opts.verify_signature, false);
        assert_eq!(opts.verify_hash, true);
        assert_eq!(opts.signature_verification_mode, VerificationMode::Warn);
        assert_eq!(opts.empty_signature_mode, VerificationMode::Silent);
        assert_eq!(opts.hash_verification_mode, VerificationMode::Warn);
    }

    #[test]
    fn test_default_values_are_applied_correctly() {
        let args = GetArgs {
            id:               "test".to_string(),
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
            wal:              WalArgs::default(),
        };

        let result = args.to_verification_options();
        assert!(result.is_ok());
        let opts = result.unwrap();

        assert_eq!(opts.verify_signature, true);
        assert_eq!(opts.verify_hash, true);
        assert_eq!(opts.signature_verification_mode, VerificationMode::Strict);
        assert_eq!(opts.empty_signature_mode, VerificationMode::Warn);
        assert_eq!(opts.hash_verification_mode, VerificationMode::Strict);
    }

    #[test]
    fn test_case_insensitive_mode_parsing() {
        let args = GetArgs {
            id:               "test".to_string(),
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "STRICT".to_string(),
            empty_sig_mode:   "WARN".to_string(),
            hash_mode:        "SILENT".to_string(),
            wal:              WalArgs::default(),
        };

        let result = args.to_verification_options();
        assert!(result.is_ok());
        let opts = result.unwrap();

        assert_eq!(opts.signature_verification_mode, VerificationMode::Strict);
        assert_eq!(opts.empty_signature_mode, VerificationMode::Warn);
        assert_eq!(opts.hash_verification_mode, VerificationMode::Silent);
    }

    #[tokio::test]
    async fn test_get_existing_document() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";
        let doc_id = "doc1";

        // Initialize store and create collection with document
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let docs = vec![(doc_id, serde_json::json!({"name": "Alice", "age": 30}))];
        collection.bulk_insert(docs).await.unwrap();

        let args = GetArgs {
            id:               doc_id.to_string(),
            verify_signature: false,
            verify_hash:      false,
            signature_mode:   "silent".to_string(),
            empty_sig_mode:   "silent".to_string(),
            hash_mode:        "silent".to_string(),
            wal:              WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_nonexistent_document() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";
        let doc_id = "nonexistent";

        // Initialize store and create collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let args = GetArgs {
            id:               doc_id.to_string(),
            verify_signature: false,
            verify_hash:      false,
            signature_mode:   "silent".to_string(),
            empty_sig_mode:   "silent".to_string(),
            hash_mode:        "silent".to_string(),
            wal:              WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_with_invalid_verification_mode() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";
        let doc_id = "doc1";

        // Initialize store and create collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let args = GetArgs {
            id:               doc_id.to_string(),
            verify_signature: false,
            verify_hash:      false,
            signature_mode:   "invalid".to_string(),
            empty_sig_mode:   "silent".to_string(),
            hash_mode:        "silent".to_string(),
            wal:              WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_invalid_store_path() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("invalid_store");
        std::fs::write(&store_path, "not a directory").unwrap();
        let collection_name = "test_collection";

        let args = GetArgs {
            id:               "doc1".to_string(),
            verify_signature: false,
            verify_hash:      false,
            signature_mode:   "silent".to_string(),
            empty_sig_mode:   "silent".to_string(),
            hash_mode:        "silent".to_string(),
            wal:              WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_invalid_document_id() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";
        let doc_id = ""; // Invalid: empty document ID

        // Initialize store and create collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let args = GetArgs {
            id:               doc_id.to_string(),
            verify_signature: false,
            verify_hash:      false,
            signature_mode:   "silent".to_string(),
            empty_sig_mode:   "silent".to_string(),
            hash_mode:        "silent".to_string(),
            wal:              WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        // Should fail due to invalid document ID
        assert!(result.is_err());
    }
}
