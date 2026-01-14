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
pub use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
pub use encrypt::Aes256GcmEncryptor;
pub use encrypt_trait::EncryptionAlgorithm;
pub use error::CryptoError;
pub use hash_trait::HashFunction;
pub use key_derivation::Blake3KeyDerivation;
pub use key_derivation_trait::KeyDerivationFunction;
// Convenience functions using default implementations
use serde_json::Value;
pub use sign::{Ed25519Signer, SigningKeyManager};
pub use sign_trait::SignatureAlgorithm;

/// Computes the Blake3 hash of the given JSON data.
pub fn hash_data(data: &Value) -> Result<String, CryptoError> { crate::hash::Blake3Hasher::hash_data(data) }

/// Signs the given hash using Ed25519.
pub fn sign_hash(hash: &str, private_key: &SigningKey) -> Result<String, CryptoError> {
    Ed25519Signer::sign_hash(hash, private_key)
}

/// Verifies the signature of the given hash using Ed25519.
pub fn verify_signature(hash: &str, signature: &str, public_key: &VerifyingKey) -> Result<bool, CryptoError> {
    Ed25519Signer::verify_signature(hash, signature, public_key)
}

/// Encrypts data using AES-256-GCM.
pub fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<String, CryptoError> {
    Aes256GcmEncryptor::encrypt_data(data, key)
}

/// Decrypts data using AES-256-GCM.
pub fn decrypt_data(encrypted_data: &str, key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    Aes256GcmEncryptor::decrypt_data(encrypted_data, key)
}

/// Derives a 32-byte key from a passphrase.
pub fn derive_key_from_passphrase(passphrase: &str) -> [u8; 32] {
    Blake3KeyDerivation::derive_key_from_passphrase(passphrase).unwrap()
}

#[cfg(test)]
mod tests {
    use rand::random;

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
    fn test_sign_and_verify_hash() {
        let secret: [u8; 32] = random();
        let private_key = SigningKey::from_bytes(&secret);
        let public_key = private_key.verifying_key();

        let hash = "some_hash_value";
        let signature = sign_hash(hash, &private_key).unwrap();

        let is_valid = verify_signature(hash, &signature, &public_key).unwrap();
        assert!(is_valid);

        let is_valid_wrong = verify_signature("wrong_hash", &signature, &public_key).unwrap();
        assert!(!is_valid_wrong);
    }
}
