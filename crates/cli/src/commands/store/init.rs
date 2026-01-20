use clap::Args;
use tracing::{error, info};
use sentinel_dbms::{CollectionWalConfig, StoreWalConfig, WalFailureMode};

use crate::commands::WalArgs;

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
    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal:                     WalArgs,
    /// Store failure mode for WAL: disabled, warn, strict (default: strict)
    #[arg(long)]
    pub wal_store_failure_mode:  Option<String>,
    /// Enable automatic checkpointing (default: true)
    #[arg(long)]
    pub wal_auto_checkpoint:     Option<bool>,
    /// Checkpoint interval in seconds (default: 300)
    #[arg(long)]
    pub wal_checkpoint_interval: Option<u64>,
    /// Maximum WAL size for store in bytes (default: 100MB)
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
///     path: String::from("/tmp/my_store"),
/// };
/// run(args).await?;
/// ```

/// Build StoreWalConfig from CLI arguments
fn build_store_wal_config(args: &InitArgs) -> StoreWalConfig {
    let default_collection_config = CollectionWalConfig {
        write_mode:            args.wal.wal_write_mode.unwrap_or(WalFailureMode::Strict),
        verification_mode:     args.wal.wal_verify_mode.unwrap_or(WalFailureMode::Warn),
        auto_verify:           args.wal.wal_auto_verify.unwrap_or(false),
        enable_recovery:       args.wal.wal_enable_recovery.unwrap_or(true),
        max_wal_size_bytes:    args.wal.wal_max_file_size,
        compression_algorithm: args.wal.wal_compression,
        max_records_per_file:  args.wal.wal_max_records,
        format:                args.wal.wal_format.unwrap_or_default(),
    };

    StoreWalConfig {
        default_collection_config,
        collection_configs: std::collections::HashMap::new(),
        store_failure_mode: args
            .wal_store_failure_mode
            .as_ref()
            .and_then(|s| parse_wal_failure_mode(s))
            .unwrap_or(WalFailureMode::Strict),
        auto_checkpoint: args.wal_auto_checkpoint.unwrap_or(true),
        checkpoint_interval_secs: args.wal_checkpoint_interval.unwrap_or(300),
        max_wal_size_bytes: args.wal_store_max_size.unwrap_or(100 * 1024 * 1024), // 100MB default
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

pub async fn run(args: InitArgs) -> sentinel_dbms::Result<()> {
    let wal_config = build_store_wal_config(&args);
    let path = args.path;
    info!("Initializing store at {}", path);

    let passphrase = args.passphrase.as_deref();
    match sentinel_dbms::Store::new_with_config(&path, passphrase, wal_config).await {
        Ok(mut store) => {
            #[allow(clippy::pattern_type_mismatch, reason = "false positive")]
            if let Some(hex) = &args.signing_key {
                let key = sentinel_dbms::SigningKeyManager::import_key(hex)?;
                store.set_signing_key(key.clone());
                if let Some(pass) = passphrase {
                    let (salt, encryption_key) = sentinel_dbms::derive_key_from_passphrase(pass).await?;
                    let encrypted = sentinel_dbms::encrypt_data(&key.to_bytes(), &encryption_key).await?;
                    let salt_hex = hex::encode(&salt);
                    let keys_collection = store.collection_with_config(".keys", None).await?;
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
            path:                    store_path.to_string_lossy().to_string(),
            passphrase:              None,
            signing_key:             None,
            wal:                     WalArgs::default(),
            wal_store_failure_mode:  None,
            wal_auto_checkpoint:     None,
            wal_checkpoint_interval: None,
            wal_store_max_size:      None,
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
            path:                    file_path.to_string_lossy().to_string(),
            passphrase:              None,
            signing_key:             None,
            wal:                     WalArgs::default(),
            wal_store_failure_mode:  None,
            wal_auto_checkpoint:     None,
            wal_checkpoint_interval: None,
            wal_store_max_size:      None,
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
            path:                    store_path.to_string_lossy().to_string(),
            passphrase:              None,
            signing_key:             None,
            wal:                     WalArgs::default(),
            wal_store_failure_mode:  None,
            wal_auto_checkpoint:     None,
            wal_checkpoint_interval: None,
            wal_store_max_size:      None,
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
            path:                    store_path.to_string_lossy().to_string(),
            passphrase:              None,
            signing_key:             None,
            wal:                     WalArgs::default(),
            wal_store_failure_mode:  None,
            wal_auto_checkpoint:     None,
            wal_checkpoint_interval: None,
            wal_store_max_size:      None,
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
        let key = sentinel_dbms::SigningKeyManager::generate_key();
        let key_hex = sentinel_dbms::SigningKeyManager::export_key(&key);

        let args = InitArgs {
            path:                    store_path.to_string_lossy().to_string(),
            passphrase:              Some(String::from("test_passphrase")),
            signing_key:             Some(key_hex),
            wal:                     WalArgs::default(),
            wal_store_failure_mode:  None,
            wal_auto_checkpoint:     None,
            wal_checkpoint_interval: None,
            wal_store_max_size:      None,
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Init with signing key should succeed");
    }
}
