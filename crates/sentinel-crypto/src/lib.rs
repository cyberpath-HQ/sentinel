//! # Sentinel Crypto
//!
//! A modular, secure cryptographic library for the Sentinel document database.
//! This crate provides hashing and digital signature operations with a focus
//! on maintainability, security, and performance.
//!
//! ## Design Principles
//!
//! - **Modular Architecture**: Traits are separated from implementations, allowing easy algorithm
//!   switching and testing.
//! - **Security First**: All sensitive data is automatically zeroized. Sealed traits prevent
//!   external insecure implementations.
//! - **Unified Error Handling**: Single `CryptoError` enum for consistent error handling across all
//!   operations.
//! - **RustCrypto Only**: Uses only audited rustcrypto crates (blake3, ed25519-dalek) for
//!   cryptographic primitives.
//! - **Parallel Performance**: BLAKE3 supports parallel computation for large inputs.
//!
//! ## Security Features
//!
//! - **Memory Protection**: `SigningKey` and other sensitive types automatically zeroize memory
//!   when dropped.
//! - **Sealed Traits**: Prevents external implementations that might bypass security.
//! - **Type Safety**: Associated types ensure compile-time correctness.
//! - **Error Abstraction**: Errors don't leak sensitive information.
//!
//! ## Performance
//!
//! - BLAKE3: High-performance hash function with parallel support.
//! - Ed25519: Fast elliptic curve signatures with 128-bit security.
//!
//! ## Usage
//!
//! ```rust
//! use sentinel_crypto::{hash_data, sign_hash, verify_signature, SigningKey};
//!
//! let data = serde_json::json!({"key": "value"});
//! let hash = hash_data(&data).unwrap();
//!
//! let key = SigningKey::from_bytes(&rand::random::<[u8; 32]>());
//! let signature = sign_hash(&hash, &key).unwrap();
//!
//! let public_key = key.verifying_key();
//! assert!(verify_signature(&hash, &signature, &public_key).unwrap());
//! ```

pub mod crypto_config;
pub mod encrypt;
pub mod encrypt_trait;
pub mod error;
pub mod hash;
pub mod hash_trait;
pub mod key_derivation;
pub mod key_derivation_trait;
pub mod sign;
pub mod sign_trait;

// Re-export crypto types for convenience
pub use crypto_config::*;
pub use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
pub use encrypt::{Aes256GcmSivEncryptor, Ascon128Encryptor, EncryptionKeyManager, XChaCha20Poly1305Encryptor};
pub use encrypt_trait::EncryptionAlgorithm;
pub use error::CryptoError;
pub use hash_trait::HashFunction;
pub use key_derivation::{Argon2KeyDerivation, Pbkdf2KeyDerivation};
pub use key_derivation_trait::KeyDerivationFunction;
// Convenience functions using default implementations
use serde_json::Value;
pub use sign::{Ed25519Signer, SigningKeyManager};
pub use sign_trait::SignatureAlgorithm;
use tracing::{debug, trace};

/// Computes the hash of the given JSON data using the globally configured algorithm.
pub async fn hash_data(data: &Value) -> Result<String, CryptoError> {
    trace!("Hashing data using global config");
    let config = get_global_crypto_config().await?;
    let result = match config.hash_algorithm {
        HashAlgorithmChoice::Blake3 => crate::hash::Blake3Hasher::hash_data(data),
    };
    if let Ok(ref hash) = result {
        debug!("Data hashed successfully: {}", hash);
    }
    result
}

/// Signs the given hash using the globally configured algorithm.
pub async fn sign_hash(hash: &str, private_key: &SigningKey) -> Result<String, CryptoError> {
    trace!("Signing hash using global config");
    let config = get_global_crypto_config().await?;
    let result = match config.signature_algorithm {
        SignatureAlgorithmChoice::Ed25519 => Ed25519Signer::sign_hash(hash, private_key),
    };
    if let Ok(ref sig) = result {
        debug!("Hash signed successfully: {}", sig);
    }
    result
}

/// Verifies the signature of the given hash using the globally configured algorithm.
pub async fn verify_signature(hash: &str, signature: &str, public_key: &VerifyingKey) -> Result<bool, CryptoError> {
    trace!("Verifying signature using global config");
    let config = get_global_crypto_config().await?;
    let result = match config.signature_algorithm {
        SignatureAlgorithmChoice::Ed25519 => Ed25519Signer::verify_signature(hash, signature, public_key),
    };
    debug!("Signature verification result: {:?}", result);
    result
}

/// Encrypts data using the globally configured algorithm.
pub async fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<String, CryptoError> {
    trace!(
        "Encrypting data using global config, data length: {}",
        data.len()
    );
    let config = get_global_crypto_config().await?;
    let result = match config.encryption_algorithm {
        EncryptionAlgorithmChoice::XChaCha20Poly1305 => XChaCha20Poly1305Encryptor::encrypt_data(data, key),
        EncryptionAlgorithmChoice::Aes256GcmSiv => Aes256GcmSivEncryptor::encrypt_data(data, key),
        EncryptionAlgorithmChoice::Ascon128 => Ascon128Encryptor::encrypt_data(data, key),
    };
    if let Ok(ref encrypted) = result {
        debug!(
            "Data encrypted successfully, encrypted length: {}",
            encrypted.len()
        );
    }
    result
}

/// Decrypts data using the globally configured algorithm.
pub async fn decrypt_data(encrypted_data: &str, key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    trace!(
        "Decrypting data using global config, encrypted length: {}",
        encrypted_data.len()
    );
    let config = get_global_crypto_config().await?;
    let result = match config.encryption_algorithm {
        EncryptionAlgorithmChoice::XChaCha20Poly1305 => XChaCha20Poly1305Encryptor::decrypt_data(encrypted_data, key),
        EncryptionAlgorithmChoice::Aes256GcmSiv => Aes256GcmSivEncryptor::decrypt_data(encrypted_data, key),
        EncryptionAlgorithmChoice::Ascon128 => Ascon128Encryptor::decrypt_data(encrypted_data, key),
    };
    if let Ok(ref decrypted) = result {
        debug!(
            "Data decrypted successfully, plaintext length: {}",
            decrypted.len()
        );
    }
    result
}

/// Derives a 32-byte key from a passphrase using the globally configured algorithm.
/// Returns the randomly generated salt and the derived key.
pub async fn derive_key_from_passphrase(passphrase: &str) -> Result<(Vec<u8>, [u8; 32]), CryptoError> {
    trace!("Deriving key from passphrase using global config");
    let config = get_global_crypto_config().await?;
    let result = match config.key_derivation_algorithm {
        KeyDerivationAlgorithmChoice::Argon2id => Argon2KeyDerivation::derive_key_from_passphrase(passphrase),
        KeyDerivationAlgorithmChoice::Pbkdf2 => Pbkdf2KeyDerivation::derive_key_from_passphrase(passphrase),
    };
    if result.is_ok() {
        debug!("Key derivation completed successfully");
    }
    result
}

/// Derives a 32-byte key from a passphrase using the provided salt and the globally configured
/// algorithm.
pub async fn derive_key_from_passphrase_with_salt(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], CryptoError> {
    trace!("Deriving key from passphrase with salt using global config");
    let config = get_global_crypto_config().await?;
    let result = match config.key_derivation_algorithm {
        KeyDerivationAlgorithmChoice::Argon2id => {
            Argon2KeyDerivation::derive_key_from_passphrase_with_salt(passphrase, salt)
        },
        KeyDerivationAlgorithmChoice::Pbkdf2 => {
            Pbkdf2KeyDerivation::derive_key_from_passphrase_with_salt(passphrase, salt)
        },
    };
    if result.is_ok() {
        debug!("Key derivation with salt completed successfully");
    }
    result
}

#[cfg(test)]
mod tests {
    use tracing_subscriber;

    use super::*;

    fn init_logging() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .try_init();
    }

    #[tokio::test]
    async fn test_hash_data() {
        init_logging();
        let data = serde_json::json!({"key": "value", "number": 42});
        let hash = hash_data(&data).await.unwrap();
        assert_eq!(hash.len(), 64);
        let hash2 = hash_data(&data).await.unwrap();
        assert_eq!(hash, hash2);
    }

    #[tokio::test]
    async fn test_set_global_crypto_config_already_set() {
        init_logging();
        reset_global_crypto_config_for_tests().await;
        let config = CryptoConfig {
            hash_algorithm:           HashAlgorithmChoice::Blake3,
            signature_algorithm:      SignatureAlgorithmChoice::Ed25519,
            encryption_algorithm:     EncryptionAlgorithmChoice::Aes256GcmSiv, // Different from default
            key_derivation_algorithm: KeyDerivationAlgorithmChoice::Argon2id,
        };

        // First set should succeed
        let result1 = set_global_crypto_config(config.clone()).await;
        assert!(result1.is_ok());

        // Second set should succeed but emit a warning
        let result2 = set_global_crypto_config(config).await;
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_aes256gcm_siv_encryption() {
        init_logging();
        reset_global_crypto_config_for_tests().await;
        let config = CryptoConfig {
            hash_algorithm:           HashAlgorithmChoice::Blake3,
            signature_algorithm:      SignatureAlgorithmChoice::Ed25519,
            encryption_algorithm:     EncryptionAlgorithmChoice::Aes256GcmSiv,
            key_derivation_algorithm: KeyDerivationAlgorithmChoice::Argon2id,
        };
        let _ = set_global_crypto_config(config).await; // Ignore if already set

        let key = [0u8; 32];
        let test_data = b"test data";
        let encrypted = encrypt_data(test_data, &key).await.unwrap();
        let decrypted = decrypt_data(&encrypted, &key).await.unwrap();
        assert_eq!(test_data, decrypted.as_slice());
    }

    #[tokio::test]
    async fn test_ascon128_encryption() {
        init_logging();
        reset_global_crypto_config_for_tests().await;
        let config = CryptoConfig {
            hash_algorithm:           HashAlgorithmChoice::Blake3,
            signature_algorithm:      SignatureAlgorithmChoice::Ed25519,
            encryption_algorithm:     EncryptionAlgorithmChoice::Ascon128,
            key_derivation_algorithm: KeyDerivationAlgorithmChoice::Argon2id,
        };
        let _ = set_global_crypto_config(config).await; // Ignore if already set

        let key = [0u8; 32];
        let test_data = b"test data";
        let encrypted = encrypt_data(test_data, &key).await.unwrap();
        let decrypted = decrypt_data(&encrypted, &key).await.unwrap();
        assert_eq!(test_data, decrypted.as_slice());
    }

    #[tokio::test]
    async fn test_pbkdf2_key_derivation() {
        init_logging();
        reset_global_crypto_config_for_tests().await;
        let config = CryptoConfig {
            hash_algorithm:           HashAlgorithmChoice::Blake3,
            signature_algorithm:      SignatureAlgorithmChoice::Ed25519,
            encryption_algorithm:     EncryptionAlgorithmChoice::XChaCha20Poly1305,
            key_derivation_algorithm: KeyDerivationAlgorithmChoice::Pbkdf2,
        };
        let _ = set_global_crypto_config(config).await; // Ignore if already set

        let derived = derive_key_from_passphrase("test passphrase").await.unwrap();
        assert_eq!(derived.1.len(), 32);
        assert_eq!(derived.0.len(), 32);
    }

    #[tokio::test]
    async fn test_sign_and_verify_hash() {
        init_logging();
        let data = serde_json::json!({"key": "value"});
        let hash = hash_data(&data).await.unwrap();
        assert_eq!(hash.len(), 64);

        let key_bytes = [0u8; 32];
        let key = SigningKey::from_bytes(&key_bytes);
        let signature = sign_hash(&hash, &key).await.unwrap();

        let public_key = key.verifying_key();
        let verified = verify_signature(&hash, &signature, &public_key)
            .await
            .unwrap();
        assert!(verified);

        // Test invalid signature (wrong hash)
        let invalid_verified = verify_signature("wrong_hash", &signature, &public_key)
            .await
            .unwrap();
        assert!(!invalid_verified);

        // Test invalid hex
        let hex_error = verify_signature(&hash, "invalid", &public_key).await;
        assert!(hex_error.is_err());
    }

    #[tokio::test]
    async fn test_decrypt_corrupted_data() {
        init_logging();
        reset_global_crypto_config_for_tests().await;
        let config = CryptoConfig {
            hash_algorithm:           HashAlgorithmChoice::Blake3,
            signature_algorithm:      SignatureAlgorithmChoice::Ed25519,
            encryption_algorithm:     EncryptionAlgorithmChoice::XChaCha20Poly1305,
            key_derivation_algorithm: KeyDerivationAlgorithmChoice::Argon2id,
        };
        let _ = set_global_crypto_config(config).await;

        let key = [0u8; 32];
        let test_data = b"test data";
        let encrypted = encrypt_data(test_data, &key).await.unwrap();

        // Corrupt the encrypted data
        let mut corrupted = encrypted.clone();
        if let Some(last) = corrupted.chars().last() {
            let new_last = if last == 'a' { 'b' } else { 'a' };
            corrupted = corrupted[.. encrypted.len() - 1].to_string() + &new_last.to_string();
        }

        let result = decrypt_data(&corrupted, &key).await;
        assert!(result.is_err());
        // Should be Decryption error
    }

    #[tokio::test]
    async fn test_verify_signature_invalid_hex() {
        init_logging();
        let data = serde_json::json!({"key": "value"});
        let hash = hash_data(&data).await.unwrap();

        let key_bytes = [0u8; 32];
        let key = SigningKey::from_bytes(&key_bytes);

        // Test with invalid hex
        let result = verify_signature(&hash, "invalid_hex", &key.verifying_key()).await;
        assert!(result.is_err());
        // Should be Hex error
    }

    #[tokio::test]
    async fn test_verify_signature_wrong_signature() {
        init_logging();
        let data = serde_json::json!({"key": "value"});
        let hash = hash_data(&data).await.unwrap();

        let key_bytes = [0u8; 32];
        let key = SigningKey::from_bytes(&key_bytes);
        let signature = sign_hash(&hash, &key).await.unwrap();

        let wrong_key_bytes = [1u8; 32];
        let wrong_key = SigningKey::from_bytes(&wrong_key_bytes);

        let result = verify_signature(&hash, &signature, &wrong_key.verifying_key()).await;
        assert!(matches!(result, Ok(false)));
        // Verification should fail
    }

    #[tokio::test]
    async fn test_derive_key_from_passphrase_with_empty_passphrase() {
        init_logging();
        reset_global_crypto_config_for_tests().await;
        let config = CryptoConfig {
            hash_algorithm:           HashAlgorithmChoice::Blake3,
            signature_algorithm:      SignatureAlgorithmChoice::Ed25519,
            encryption_algorithm:     EncryptionAlgorithmChoice::XChaCha20Poly1305,
            key_derivation_algorithm: KeyDerivationAlgorithmChoice::Argon2id,
        };
        let _ = set_global_crypto_config(config).await;

        let passphrase = "";
        let salt = b"salt";

        let result = derive_key_from_passphrase_with_salt(passphrase, salt).await;
        // Empty passphrase should fail
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_decrypt_short_ciphertext() {
        init_logging();
        reset_global_crypto_config_for_tests().await;
        let config = CryptoConfig {
            hash_algorithm:           HashAlgorithmChoice::Blake3,
            signature_algorithm:      SignatureAlgorithmChoice::Ed25519,
            encryption_algorithm:     EncryptionAlgorithmChoice::XChaCha20Poly1305,
            key_derivation_algorithm: KeyDerivationAlgorithmChoice::Argon2id,
        };
        let _ = set_global_crypto_config(config).await;

        let key = [0u8; 32];
        let short_ciphertext = "short";

        let result = decrypt_data(short_ciphertext, &key).await;
        assert!(result.is_err());
        // Should be Decryption error
    }
}
