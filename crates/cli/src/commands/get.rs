use std::str::FromStr as _;

use clap::Args;
use sentinel_dbms::{CollectionWalConfig, VerificationMode, VerificationOptions};
use sentinel_wal::{manager::WalFormat, CompressionAlgorithm, WalFailureMode};
use tracing::{error, info, warn};

/// Arguments for get command.
#[derive(Args, Clone, Default)]
pub struct GetArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path:          String,
    /// Collection name
    #[arg(short, long)]
    pub collection:          String,
    /// Document ID
    #[arg(short, long)]
    pub id:                  String,
    /// Passphrase for decrypting signing key
    #[arg(long)]
    pub passphrase:          Option<String>,
    /// Verify document signature (default: true)
    #[arg(long, default_value = "true")]
    pub verify_signature:    bool,
    /// Verify document hash (default: true)
    #[arg(long, default_value = "true")]
    pub verify_hash:         bool,
    /// Signature verification mode: strict, warn, or silent (default: strict)
    #[arg(long, default_value = "strict")]
    pub signature_mode:      String,
    /// How to handle documents with no signature: strict, warn, or silent (default: warn)
    #[arg(long, default_value = "warn")]
    pub empty_sig_mode:      String,
    /// Hash verification mode: strict, warn, or silent (default: strict)
    #[arg(long, default_value = "strict")]
    pub hash_mode:           String,
    /// Maximum WAL file size in bytes for this collection (default: 10MB)
    #[arg(long)]
    pub wal_max_file_size:   Option<u64>,
    /// WAL file format for this collection: binary or json_lines (default: binary)
    #[arg(long)]
    pub wal_format:          Option<String>,
    /// WAL compression algorithm for this collection: zstd, lz4, brotli, deflate, gzip (default:
    /// zstd)
    #[arg(long)]
    pub wal_compression:     Option<String>,
    /// Maximum number of records per WAL file for this collection (default: 1000)
    #[arg(long)]
    pub wal_max_records:     Option<usize>,
    /// WAL write mode for this collection: disabled, warn, strict (default: strict)
    #[arg(long)]
    pub wal_write_mode:      Option<String>,
    /// WAL verification mode for this collection: disabled, warn, strict (default: warn)
    #[arg(long)]
    pub wal_verify_mode:     Option<String>,
    /// Enable automatic document verification against WAL for this collection (default: false)
    #[arg(long)]
    pub wal_auto_verify:     Option<bool>,
    /// Enable WAL-based recovery features for this collection (default: true)
    #[arg(long)]
    pub wal_enable_recovery: Option<bool>,
}

impl GetArgs {
    /// Convert CLI arguments to verification options.
    fn to_verification_options(&self) -> Result<VerificationOptions, String> {
        let signature_verification_mode = VerificationMode::from_str(&self.signature_mode).map_err(|_e| {
            format!(
                "Invalid signature verification mode: {}",
                self.signature_mode
            )
        })?;

        let empty_signature_mode = VerificationMode::from_str(&self.empty_sig_mode)
            .map_err(|_e| format!("Invalid empty signature mode: {}", self.empty_sig_mode))?;

        let hash_verification_mode = VerificationMode::from_str(&self.hash_mode)
            .map_err(|_e| format!("Invalid hash verification mode: {}", self.hash_mode))?;

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
/// * `args` - The parsed command-line arguments for get.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::get::{run, GetArgs};
///
/// let args = GetArgs {
///     store_path:       "/tmp/my_store".to_string(),
///     collection:       "users".to_string(),
///     id:               "user1".to_string(),
///     passphrase:       None,
///     verify_signature: true,
///     verify_hash:      true,
///     signature_mode:   "strict".to_string(),
///     hash_mode:        "strict".to_string(),
///     wal_max_file_size: None,
///     wal_format: None,
///     wal_compression: None,
///     wal_max_records: None,
///     wal_write_mode: None,
///     wal_verify_mode: None,
///     wal_auto_verify: None,
///     wal_enable_recovery: None,
/// };
/// };
/// run(args).await?;
/// ```

/// Build CollectionWalConfig from CLI arguments
fn build_collection_wal_config(args: &GetArgs) -> Option<CollectionWalConfig> {
    // Only build config if any WAL options are provided
    if args.wal_max_file_size.is_some() ||
        args.wal_format.is_some() ||
        args.wal_compression.is_some() ||
        args.wal_max_records.is_some() ||
        args.wal_write_mode.is_some() ||
        args.wal_verify_mode.is_some() ||
        args.wal_auto_verify.is_some() ||
        args.wal_enable_recovery.is_some()
    {
        Some(CollectionWalConfig {
            write_mode:            args
                .wal_write_mode
                .as_ref()
                .and_then(|s| parse_wal_failure_mode(s))
                .unwrap_or(WalFailureMode::Strict),
            verification_mode:     args
                .wal_verify_mode
                .as_ref()
                .and_then(|s| parse_wal_failure_mode(s))
                .unwrap_or(WalFailureMode::Warn),
            auto_verify:           args.wal_auto_verify.unwrap_or(false),
            enable_recovery:       args.wal_enable_recovery.unwrap_or(true),
            max_wal_size_bytes:    args.wal_max_file_size,
            compression_algorithm: args
                .wal_compression
                .as_ref()
                .and_then(|s| parse_compression_algorithm(s)),
            max_records_per_file:  args.wal_max_records,
            format:                args
                .wal_format
                .as_ref()
                .and_then(|s| parse_wal_format(s))
                .unwrap_or_default(),
        })
    }
    else {
        None
    }
}

/// Parse WAL failure mode from string
fn parse_wal_failure_mode(s: &str) -> Option<WalFailureMode> {
    match s.to_lowercase().as_str() {
        "disabled" => Some(WalFailureMode::Disabled),
        "warn" => Some(WalFailureMode::Warn),
        "strict" => Some(WalFailureMode::Strict),
        _ => None,
    }
}

/// Parse compression algorithm from string
fn parse_compression_algorithm(s: &str) -> Option<CompressionAlgorithm> {
    match s.to_lowercase().as_str() {
        "zstd" => Some(CompressionAlgorithm::Zstd),
        "lz4" => Some(CompressionAlgorithm::Lz4),
        "brotli" => Some(CompressionAlgorithm::Brotli),
        "deflate" => Some(CompressionAlgorithm::Deflate),
        "gzip" => Some(CompressionAlgorithm::Gzip),
        _ => None,
    }
}

/// Parse WAL format from string
fn parse_wal_format(s: &str) -> Option<WalFormat> {
    match s.to_lowercase().as_str() {
        "binary" => Some(WalFormat::Binary),
        "json_lines" => Some(WalFormat::JsonLines),
        _ => None,
    }
}

pub async fn run(args: GetArgs) -> sentinel_dbms::Result<()> {
    let store_path = &args.store_path;
    let collection = &args.collection;
    let id = &args.id;
    info!(
        "Getting document '{}' from collection '{}' in store {}",
        id, collection, store_path
    );
    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        args.passphrase.as_deref(),
        sentinel_wal::StoreWalConfig::default(),
    )
    .await?;
    let wal_config = build_collection_wal_config(&args);
    let coll = if let Some(config) = wal_config {
        store
            .collection_with_config(collection, Some(config))
            .await?
    }
    else {
        store.collection(collection).await?
    };

    let id = &args.id;
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
            warn!("Document '{}' not found", id);
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
    use tempfile::TempDir;

    use super::*;

    /// Test successful document retrieval.
    ///
    /// This test inserts a document and then retrieves it successfully.
    #[tokio::test]
    async fn test_get_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init store, create collection, insert document
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let insert_args = crate::commands::insert::InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id: Some("doc1".to_string()),
            data: Some(r#"{"name": "Alice", "age": 30}"#.to_string()),
            ..Default::default()
        };
        crate::commands::insert::run(insert_args).await.unwrap();

        // Capture stdout for testing
        {
            let args = GetArgs {
                store_path:       store_path.to_string_lossy().to_string(),
                collection:       "test_collection".to_string(),
                id:               "doc1".to_string(),
                passphrase:       None,
                verify_signature: false,
                verify_hash:      false,
                signature_mode:   "strict".to_string(),
                empty_sig_mode:   "warn".to_string(),
                hash_mode:        "strict".to_string(),
            };

            // Since run prints to stdout, we need to capture it
            // For simplicity, we'll just check that it doesn't error
            let result = run(args).await;
            assert!(result.is_ok(), "Get should succeed for existing document");
        }
    }

    /// Test get non-existent document.
    ///
    /// This test verifies that get handles the case where the document
    /// does not exist (should succeed but warn).
    #[tokio::test]
    async fn test_get_non_existent_document() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init store and collection
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let args = GetArgs {
            store_path:       store_path.to_string_lossy().to_string(),
            collection:       "test_collection".to_string(),
            id:               "non_existent".to_string(),
            passphrase:       None,
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
        };

        let result = run(args).await;
        assert!(
            result.is_ok(),
            "Get should succeed (but warn) for non-existent document"
        );
    }

    /// Test get from non-existent collection.
    ///
    /// This test checks that get creates the collection if it does not exist.
    #[tokio::test]
    async fn test_get_non_existent_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store only
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            ..Default::default()
        };
        crate::commands::init::run(init_args).await.unwrap();

        let args = GetArgs {
            store_path:       store_path.to_string_lossy().to_string(),
            collection:       "non_existent".to_string(),
            id:               "doc1".to_string(),
            passphrase:       None,
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Get should create collection if needed");
    }

    /// Test get with empty ID.
    ///
    /// This test verifies behavior when an empty document ID is provided.
    #[tokio::test]
    async fn test_get_empty_id() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init store and collection
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let args = GetArgs {
            store_path:       store_path.to_string_lossy().to_string(),
            collection:       "test_collection".to_string(),
            id:               "".to_string(),
            passphrase:       None,
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_err(), "Get with empty ID should fail validation");
    }

    /// This test verifies that get fails when the collection directory
    /// is unreadable, covering the coll.get error branch.
    #[tokio::test]
    async fn test_get_unreadable_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init store and create collection
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        // Insert a document first
        let insert_args = crate::commands::insert::InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id: Some("doc1".to_string()),
            data: Some(r#"{"name": "test"}"#.to_string()),
            ..Default::default()
        };
        crate::commands::insert::run(insert_args).await.unwrap();

        // Make the collection directory unreadable (no read permission)
        let collection_path = store_path.join("data").join("test_collection");
        let mut perms = std::fs::metadata(&collection_path).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o000); // No permissions
        }
        std::fs::set_permissions(&collection_path, perms).unwrap();

        let args = GetArgs {
            store_path:       store_path.to_string_lossy().to_string(),
            collection:       "test_collection".to_string(),
            id:               "doc1".to_string(),
            passphrase:       None,
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_err(), "Get should fail on unreadable collection");

        // Restore permissions for cleanup
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&collection_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&collection_path, perms).unwrap();
        }
    }

    #[tokio::test]
    async fn test_get_invalid_signature_mode() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        let args = GetArgs {
            store_path:       store_path.to_string_lossy().to_string(),
            collection:       "test_collection".to_string(),
            id:               "doc1".to_string(),
            passphrase:       None,
            verify_signature: true,
            verify_hash:      false,
            signature_mode:   "invalid".to_string(),
            empty_sig_mode:   "warn".to_string(),
            hash_mode:        "strict".to_string(),
        };

        let result = run(args).await;
        assert!(
            result.is_err(),
            "Get should fail with invalid signature mode"
        );
    }
}
