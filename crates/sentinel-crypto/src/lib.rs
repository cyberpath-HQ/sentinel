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

mod crypto_config;
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

/// Computes the hash of the given JSON data using the globally configured algorithm.
pub fn hash_data(data: &Value) -> Result<String, CryptoError> {
    match get_global_crypto_config().hash_algorithm {
        HashAlgorithmChoice::Blake3 => crate::hash::Blake3Hasher::hash_data(data),
    }
}

/// Signs the given hash using the globally configured algorithm.
pub fn sign_hash(hash: &str, private_key: &SigningKey) -> Result<String, CryptoError> {
    match get_global_crypto_config().signature_algorithm {
        SignatureAlgorithmChoice::Ed25519 => Ed25519Signer::sign_hash(hash, private_key),
    }
}

/// Verifies the signature of the given hash using the globally configured algorithm.
pub fn verify_signature(hash: &str, signature: &str, public_key: &VerifyingKey) -> Result<bool, CryptoError> {
    match get_global_crypto_config().signature_algorithm {
        SignatureAlgorithmChoice::Ed25519 => Ed25519Signer::verify_signature(hash, signature, public_key),
    }
}

/// Encrypts data using the globally configured algorithm.
pub fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<String, CryptoError> {
    match get_global_crypto_config().encryption_algorithm {
        EncryptionAlgorithmChoice::XChaCha20Poly1305 => XChaCha20Poly1305Encryptor::encrypt_data(data, key),
        EncryptionAlgorithmChoice::Aes256GcmSiv => Aes256GcmSivEncryptor::encrypt_data(data, key),
        EncryptionAlgorithmChoice::Ascon128 => Ascon128Encryptor::encrypt_data(data, key),
    }
}

/// Decrypts data using the globally configured algorithm.
pub fn decrypt_data(encrypted_data: &str, key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    match get_global_crypto_config().encryption_algorithm {
        EncryptionAlgorithmChoice::XChaCha20Poly1305 => XChaCha20Poly1305Encryptor::decrypt_data(encrypted_data, key),
        EncryptionAlgorithmChoice::Aes256GcmSiv => Aes256GcmSivEncryptor::decrypt_data(encrypted_data, key),
        EncryptionAlgorithmChoice::Ascon128 => Ascon128Encryptor::decrypt_data(encrypted_data, key),
    }
}

/// Derives a 32-byte key from a passphrase using the globally configured algorithm.
/// Returns the randomly generated salt and the derived key.
pub fn derive_key_from_passphrase(passphrase: &str) -> Result<(Vec<u8>, [u8; 32]), CryptoError> {
    match get_global_crypto_config().key_derivation_algorithm {
        KeyDerivationAlgorithmChoice::Argon2id => Argon2KeyDerivation::derive_key_from_passphrase(passphrase),
        KeyDerivationAlgorithmChoice::Pbkdf2 => Pbkdf2KeyDerivation::derive_key_from_passphrase(passphrase),
    }
}

/// Derives a 32-byte key from a passphrase using the provided salt and the globally configured algorithm.
pub fn derive_key_from_passphrase_with_salt(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], CryptoError> {
    match get_global_crypto_config().key_derivation_algorithm {
        KeyDerivationAlgorithmChoice::Argon2id => Argon2KeyDerivation::derive_key_from_passphrase_with_salt(passphrase, salt),
        KeyDerivationAlgorithmChoice::Pbkdf2 => Pbkdf2KeyDerivation::derive_key_from_passphrase_with_salt(passphrase, salt),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_data() {
        let data = serde_json::json!({"key": "value", "number": 42});
        let hash = hash_data(&data).unwrap();
        assert_eq!(hash.len(), 64);
        let hash2 = hash_data(&data).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_global_config() {
        // Test default configuration
        let config = get_global_crypto_config();
        assert!(matches!(config.hash_algorithm, HashAlgorithmChoice::Blake3));
        assert!(matches!(
            config.signature_algorithm,
            SignatureAlgorithmChoice::Ed25519
        ));
        assert!(matches!(
            config.encryption_algorithm,
            EncryptionAlgorithmChoice::XChaCha20Poly1305
        ));
        assert!(matches!(
            config.key_derivation_algorithm,
            KeyDerivationAlgorithmChoice::Argon2id
        ));

        // Test that default functions work with the default configuration
        let data = serde_json::json!({"test": "data"});
        let hash = hash_data(&data).unwrap();
        assert_eq!(hash.len(), 64); // Blake3 hash

        let key = [0u8; 32];
        let test_data = b"test data";
        let encrypted = encrypt_data(test_data, &key).unwrap();
        let decrypted = decrypt_data(&encrypted, &key).unwrap();
        assert_eq!(test_data, decrypted.as_slice()); // Should work with XChaCha20-Poly1305

        let derived = derive_key_from_passphrase("test passphrase").unwrap();
        assert_eq!(derived.1.len(), 32); // Should work with Argon2id
        assert_eq!(derived.0.len(), 32); // Salt should be 32 bytes
    }
}
