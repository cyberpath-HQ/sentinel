use clap::Args;
use sentinel_crypto::{
    set_global_crypto_config,
    CryptoConfig,
    EncryptionAlgorithmChoice,
    HashAlgorithmChoice,
    KeyDerivationAlgorithmChoice,
    SignatureAlgorithmChoice,
};
use tracing::{error, info};

impl Default for InitArgs {
    fn default() -> Self {
        Self {
            path:                     String::new(),
            passphrase:               None,
            signing_key:              None,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        }
    }
}

/// Parse hash algorithm string to enum
fn parse_hash_algorithm(s: &str) -> Result<HashAlgorithmChoice, String> {
    match s {
        "blake3" => Ok(HashAlgorithmChoice::Blake3),
        _ => Err(format!("Invalid hash algorithm: {}", s)),
    }
}

/// Parse signature algorithm string to enum
fn parse_signature_algorithm(s: &str) -> Result<SignatureAlgorithmChoice, String> {
    match s {
        "ed25519" => Ok(SignatureAlgorithmChoice::Ed25519),
        _ => Err(format!("Invalid signature algorithm: {}", s)),
    }
}

/// Parse encryption algorithm string to enum
fn parse_encryption_algorithm(s: &str) -> Result<EncryptionAlgorithmChoice, String> {
    match s {
        "xchacha20poly1305" => Ok(EncryptionAlgorithmChoice::XChaCha20Poly1305),
        "aes256gcmsiv" => Ok(EncryptionAlgorithmChoice::Aes256GcmSiv),
        "ascon128" => Ok(EncryptionAlgorithmChoice::Ascon128),
        _ => Err(format!("Invalid encryption algorithm: {}", s)),
    }
}

/// Parse key derivation algorithm string to enum
fn parse_key_derivation_algorithm(s: &str) -> Result<KeyDerivationAlgorithmChoice, String> {
    match s {
        "argon2id" => Ok(KeyDerivationAlgorithmChoice::Argon2id),
        "pbkdf2" => Ok(KeyDerivationAlgorithmChoice::Pbkdf2),
        _ => Err(format!("Invalid key derivation algorithm: {}", s)),
    }
}

/// Arguments for the init command.
#[derive(Args, Clone)]
pub struct InitArgs {
    /// Path to the store directory
    #[arg(short, long)]
    pub path:                     String,
    /// Passphrase for encrypting the signing key
    #[arg(long)]
    pub passphrase:               Option<String>,
    /// Signing key to use (hex-encoded). If not provided, a new one is generated.
    #[arg(long)]
    pub signing_key:              Option<String>,
    /// Hash algorithm to use for cryptographic operations & data integrity.
    ///
    /// Options:
    /// - blake3 (fast, secure, default)
    #[arg(long, value_name = "ALGORITHM", default_value = "blake3", value_parser = ["blake3"], verbatim_doc_comment)]
    pub hash_algorithm:           String,
    /// Signature algorithm to use for cryptographic operations & authentication.
    ///
    /// Options:
    /// - ed25519 (secure, performant, default)
    #[arg(long, value_name = "ALGORITHM", default_value = "ed25519", value_parser = ["ed25519"], verbatim_doc_comment)]
    pub signature_algorithm:      String,
    /// Encryption algorithm to use for cryptographic operations & data protection.
    ///
    /// Options:
    /// - xchacha20poly1305 (strongest security, nonce-misuse resistant, default)
    /// - aes256gcmsiv (strong security, nonce-misuse resistant)
    /// - ascon128 (lightweight, good security for constrained environments)
    #[arg(long, value_name = "ALGORITHM", default_value = "xchacha20poly1305", value_parser = ["xchacha20poly1305", "aes256gcmsiv", "ascon128"], verbatim_doc_comment)]
    pub encryption_algorithm:     String,
    /// Key derivation algorithm to use for cryptographic operations & passphrase-based key generation.
    ///
    /// Options:
    /// - argon2id (strong security against attacks, default)
    /// - pbkdf2 (widely supported for constrained environments)
    #[arg(long, value_name = "ALGORITHM", default_value = "argon2id", value_parser = ["argon2id", "pbkdf2"], verbatim_doc_comment)]
    pub key_derivation_algorithm: String,
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

    // Set global crypto configuration
    let hash_alg = parse_hash_algorithm(&args.hash_algorithm).map_err(|e| {
        sentinel::SentinelError::ConfigError {
            message: e,
        }
    })?;
    let sig_alg = parse_signature_algorithm(&args.signature_algorithm).map_err(|e| {
        sentinel::SentinelError::ConfigError {
            message: e,
        }
    })?;
    let enc_alg = parse_encryption_algorithm(&args.encryption_algorithm).map_err(|e| {
        sentinel::SentinelError::ConfigError {
            message: e,
        }
    })?;
    let kd_alg = parse_key_derivation_algorithm(&args.key_derivation_algorithm).map_err(|e| {
        sentinel::SentinelError::ConfigError {
            message: e,
        }
    })?;

    let config = CryptoConfig {
        hash_algorithm:           hash_alg.clone(),
        signature_algorithm:      sig_alg.clone(),
        encryption_algorithm:     enc_alg.clone(),
        key_derivation_algorithm: kd_alg.clone(),
    };

    set_global_crypto_config(config.clone())
        .map_err(|_| {
            sentinel::SentinelError::ConfigError {
                message: "Crypto config already set".to_string(),
            }
        })
        .or_else(|_| {
            // If already set, check if it's the same config
            let current = sentinel_crypto::get_global_crypto_config();
            if *current == config {
                Ok(())
            }
            else {
                Err(sentinel::SentinelError::ConfigError {
                    message: "Crypto config already set with different values".to_string(),
                })
            }
        })?;

    let passphrase = args.passphrase.as_deref();
    match sentinel::Store::new(&path, passphrase).await {
        Ok(mut store) => {
            #[allow(clippy::pattern_type_mismatch, reason = "false positive")]
            if let Some(hex) = &args.signing_key {
                let key = sentinel_crypto::SigningKeyManager::import_key(hex)?;
                store.set_signing_key(key.clone());
                if let Some(pass) = passphrase {
                    let (salt, encryption_key) = sentinel_crypto::derive_key_from_passphrase(pass)?;
                    let encrypted = sentinel_crypto::encrypt_data(&key.to_bytes(), &encryption_key)?;
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
            path:                     store_path.to_string_lossy().to_string(),
            passphrase:               None,
            signing_key:              None,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
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
            path:                     file_path.to_string_lossy().to_string(),
            passphrase:               None,
            signing_key:              None,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
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
            path:                     store_path.to_string_lossy().to_string(),
            passphrase:               None,
            signing_key:              None,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
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
            path:                     store_path.to_string_lossy().to_string(),
            passphrase:               None,
            signing_key:              None,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
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
            path:                     store_path.to_string_lossy().to_string(),
            passphrase:               Some("test_passphrase".to_string()),
            signing_key:              Some(key_hex),
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Init with signing key should succeed");
    }
}
