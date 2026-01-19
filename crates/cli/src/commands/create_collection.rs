use clap::Args;
use tracing::{error, info};
use sentinel_dbms::{CollectionWalConfig, CompressionAlgorithm, WalFailureMode, WalFormat};

/// Arguments for the create-collection command.
#[derive(Args, Clone, Default)]
pub struct CreateCollectionArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path:          String,
    /// Collection name
    #[arg(short, long)]
    pub name:                String,
    /// Passphrase for decrypting the signing key
    #[arg(long)]
    pub passphrase:          Option<String>,
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

/// Create a new collection within an existing Sentinel store.
///
/// This function creates a logical grouping for documents within the specified store.
/// It validates that the store exists and handles any errors during collection creation.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for create-collection.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::create_collection::{
///     run,
///     CreateCollectionArgs,
/// };
///
/// let args = CreateCollectionArgs {
///     store_path: "/tmp/my_store".to_string(),
///     name:       "users".to_string(),
/// };
/// run(args).await?;
/// ```

/// Build CollectionWalConfig from CLI arguments
fn build_collection_wal_config(args: &CreateCollectionArgs) -> CollectionWalConfig {
    CollectionWalConfig {
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

pub async fn run(args: CreateCollectionArgs) -> sentinel_dbms::Result<()> {
    let wal_config = build_collection_wal_config(&args);
    let store_path = args.store_path;
    let name = args.name;
    info!("Creating collection '{}' in store {}", name, store_path);
    // For create_collection, we need to open the store with its existing config
    // and then create the collection with the specified WAL config
    let store = sentinel_dbms::Store::new(&store_path, args.passphrase.as_deref()).await?;
    match store.collection_with_config(&name, Some(wal_config)).await {
        Ok(_) => {
            info!("Collection '{}' created successfully", name);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to create collection '{}' in store {}: {}",
                name, store_path, e
            );
            Err(e)
        },
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    /// Test successful collection creation.
    ///
    /// This test verifies that create-collection succeeds when given a valid
    /// store path and collection name. It first initializes a store, then creates
    /// a collection within it.
    #[tokio::test]
    async fn test_create_collection_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // First init the store
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        crate::commands::init::run(init_args).await.unwrap();

        let args = CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Create collection should succeed");
    }

    /// Test create collection with non-existent store.
    ///
    /// This test checks that create-collection creates the store if it doesn't exist.
    #[tokio::test]
    async fn test_create_collection_non_existent_store() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("non_existent_store");

        let args = CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };

        let result = run(args).await;
        assert!(
            result.is_ok(),
            "Create collection should create store if needed"
        );
    }

    /// Test create collection with invalid collection name.
    ///
    /// This test verifies behavior with potentially invalid collection names,
    /// such as empty strings or names with special characters.
    #[tokio::test]
    async fn test_create_collection_invalid_name() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        crate::commands::init::run(init_args).await.unwrap();

        // Test with empty name
        let args = CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "".to_string(),
            ..Default::default()
        };

        let result = run(args).await;
        // Empty name should be rejected
        assert!(
            result.is_err(),
            "Create collection with empty name should fail validation"
        );
    }

    /// Test create collection with read-only store.
    ///
    /// This test verifies that create-collection fails when the store directory
    /// is read-only, covering the error branch.
    #[tokio::test]
    async fn test_create_collection_readonly_store() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("readonly_store");

        // Init store
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        crate::commands::init::run(init_args).await.unwrap();

        // Make the store directory read-only
        let mut perms = std::fs::metadata(&store_path).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&store_path, perms).unwrap();

        let args = CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };

        let result = run(args).await;
        assert!(
            result.is_err(),
            "Create collection should fail on read-only store"
        );

        // Restore permissions for cleanup
        let mut perms = std::fs::metadata(&store_path).unwrap().permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(&store_path, perms).unwrap();
    }
}
