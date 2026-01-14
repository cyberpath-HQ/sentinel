use clap::Args;
use sentinel_crypto;
use tracing::{error, info};

/// Arguments for the init command.
#[derive(Args, Clone, Default)]
pub struct InitArgs {
    /// Path to the store directory
    #[arg(short, long)]
    pub path:        String,
    /// Passphrase for encrypting the signing key
    #[arg(long)]
    pub passphrase:  Option<String>,
    /// Signing key to use (hex-encoded). If not provided, a new one is generated.
    #[arg(long)]
    pub signing_key: Option<String>,
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
pub async fn run(args: InitArgs) -> sentinel::Result<()> {
    let path = args.path;
    info!("Initializing store at {}", path);
    let passphrase = args.passphrase.as_deref();
    match sentinel::Store::new(&path, passphrase).await {
        Ok(mut store) => {
            if let Some(hex) = &args.signing_key {
                let key = sentinel_crypto::SigningKeyManager::import_key(hex)?;
                store.set_signing_key(key.clone());
                if let Some(pass) = passphrase {
                    let encryption_key = sentinel_crypto::derive_key_from_passphrase(pass);
                    let encrypted = sentinel_crypto::encrypt_data(&key.to_bytes(), &encryption_key)?;
                    let keys_collection = store.collection("_keys").await?;
                    keys_collection
                        .insert("signing_key", serde_json::json!({"encrypted": encrypted}))
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
            path:        store_path.to_string_lossy().to_string(),
            passphrase:  None,
            signing_key: None,
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
            path:        file_path.to_string_lossy().to_string(),
            passphrase:  None,
            signing_key: None,
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
            path:        store_path.to_string_lossy().to_string(),
            passphrase:  None,
            signing_key: None,
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
            path:        store_path.to_string_lossy().to_string(),
            passphrase:  None,
            signing_key: None,
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
            path:        store_path.to_string_lossy().to_string(),
            passphrase:  Some("test_passphrase".to_string()),
            signing_key: Some(key_hex),
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Init with signing key should succeed");
    }
}
