use clap::Args;
use tracing::{error, info};

/// Arguments for the init command.
#[derive(Args, Clone, Default)]
pub struct InitArgs {
    /// Path to the store directory
    #[arg(short, long)]
    pub path:                    String,
    /// Passphrase for encrypting the signing key
    #[arg(long)]
    pub passphrase:              Option<String>,
    /// Signing key to use (hex-encoded). If not provided, a new one is generated.
    #[arg(long)]
    pub signing_key:             Option<String>,
    /// Maximum WAL file size in bytes (default: 10MB)
    #[arg(long)]
    pub wal_max_file_size:       Option<u64>,
    /// WAL file format: binary or json_lines (default: binary)
    #[arg(long, value_enum)]
    pub wal_format:              Option<String>,
    /// WAL compression algorithm: zstd, lz4, brotli, deflate, gzip (default: zstd)
    #[arg(long, value_enum)]
    pub wal_compression:         Option<String>,
    /// Maximum number of records per WAL file (default: 1000)
    #[arg(long)]
    pub wal_max_records:         Option<usize>,
    /// WAL write mode: disabled, warn, strict (default: strict)
    #[arg(long, value_enum)]
    pub wal_write_mode:          Option<String>,
    /// WAL verification mode: disabled, warn, strict (default: warn)
    #[arg(long, value_enum)]
    pub wal_verify_mode:         Option<String>,
    /// Enable automatic document verification against WAL (default: false)
    #[arg(long)]
    pub wal_auto_verify:         Option<bool>,
    /// Enable WAL-based recovery features (default: true)
    #[arg(long)]
    pub wal_enable_recovery:     Option<bool>,
    /// Store-level WAL failure mode: disabled, warn, strict (default: strict)
    #[arg(long, value_enum)]
    pub wal_store_failure_mode:  Option<String>,
    /// Enable automatic store-wide checkpoints (default: true)
    #[arg(long)]
    pub wal_auto_checkpoint:     Option<bool>,
    /// Interval for automatic checkpoints in seconds (default: 300)
    #[arg(long)]
    pub wal_checkpoint_interval: Option<u64>,
    /// Maximum WAL file size before forcing checkpoint in bytes (default: 100MB)
    #[arg(long)]
    pub wal_store_max_size:      Option<u64>,
}

/// Initialize a new Sentinel store at the specified path.
///
/// This function creates the necessary directory structure and metadata
/// for a new Sentinel store. It logs the operation and handles any errors
/// that may occur during initialization.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for init.
///
/// # Returns
/// Returns `Ok(())` on success, or an `io::Error` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::init::{run, InitArgs};
///
/// let args = InitArgs {
///     path: "/tmp/my_store".to_string(),
/// };
/// run(args).await?;
/// ```
pub async fn run(args: InitArgs) -> sentinel_dbms::Result<()> {
    let path = args.path;
    info!("Initializing store at {}", path);

    let passphrase = args.passphrase.as_deref();
    match sentinel_dbms::Store::new(&path, passphrase).await {
        Ok(mut store) => {
            #[allow(clippy::pattern_type_mismatch, reason = "false positive")]
            if let Some(hex) = &args.signing_key {
                let key = sentinel_crypto::SigningKeyManager::import_key(hex)?;
                store.set_signing_key(key.clone());
                if let Some(pass) = passphrase {
                    let (salt, encryption_key) = sentinel_crypto::derive_key_from_passphrase(pass).await?;
                    let encrypted = sentinel_crypto::encrypt_data(&key.to_bytes(), &encryption_key).await?;
                    let salt_hex = hex::encode(&salt);
                    let keys_collection = store.collection(".keys").await?;
                    keys_collection
                        .insert(
                            "signing_key",
                            serde_json::json!({"encrypted": encrypted, "salt": salt_hex}),
                        )
                        .await?;
                }
            }
            info!("Store initialized successfully at {}", path);
            Ok(())
        },
        Err(e) => {
            error!("Failed to initialize store at {}: {}", path, e);
            Err(e)
        },
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    /// Test successful store initialization.
    ///
    /// This test verifies that the init command successfully creates a new store
    /// at a valid path. It uses a temporary directory to avoid side effects.
    #[tokio::test]
    async fn test_init_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        let args = InitArgs {
            path: store_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            wal_max_file_size: None,
            wal_format: None,
            wal_compression: None,
            wal_max_records: None,
            ..Default::default()
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Init should succeed for valid path");

        // Verify store directory was created
        assert!(
            store_path.exists(),
            "Store directory should exist after init"
        );
    }

    /// Test init with invalid path.
    ///
    /// This test checks that init fails when the path is a file instead of a directory.
    #[tokio::test]
    async fn test_init_invalid_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("not_a_dir");

        // Create a file at the path
        std::fs::write(&file_path, "not a dir").unwrap();

        let args = InitArgs {
            path: file_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            wal_max_file_size: None,
            wal_format: None,
            wal_compression: None,
            wal_max_records: None,
            ..Default::default()
        };

        let result = run(args).await;
        // Should fail because path is a file
        assert!(result.is_err(), "Init should fail when path is a file");
    }

    /// Test init with existing directory.
    ///
    /// This test verifies that init can handle the case where the directory
    /// already exists. Sentinel should handle this gracefully.
    #[tokio::test]
    async fn test_init_existing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("existing_store");

        // Create directory first
        std::fs::create_dir(&store_path).unwrap();

        let args = InitArgs {
            path: store_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            wal_max_file_size: None,
            wal_format: None,
            wal_compression: None,
            wal_max_records: None,
            ..Default::default()
        };

        let result = run(args).await;
        // Depending on implementation, this might succeed or fail
        // For now, assume it succeeds as Store::new might handle existing dirs
        assert!(result.is_ok(), "Init should handle existing directory");
    }

    /// Test init with nested path creation.
    ///
    /// This test checks that init creates parent directories if they don't exist.
    #[tokio::test]
    async fn test_init_nested_path() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("nested").join("deep").join("store");

        let args = InitArgs {
            path: store_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            wal_max_file_size: None,
            wal_format: None,
            wal_compression: None,
            wal_max_records: None,
            ..Default::default()
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Init should create nested directories");

        assert!(store_path.exists(), "Store directory should exist");
    }

    /// Test init with signing key.
    ///
    /// This test verifies that init can handle a provided signing key.
    #[tokio::test]
    async fn test_init_with_signing_key() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("store_with_key");

        // Generate a signing key for testing
        let key = sentinel_crypto::SigningKeyManager::generate_key();
        let key_hex = sentinel_crypto::SigningKeyManager::export_key(&key);

        let args = InitArgs {
            path: store_path.to_string_lossy().to_string(),
            passphrase: Some("test_passphrase".to_string()),
            signing_key: Some(key_hex),
            wal_max_file_size: None,
            wal_format: None,
            wal_compression: None,
            wal_max_records: None,
            ..Default::default()
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Init with signing key should succeed");
    }
}
