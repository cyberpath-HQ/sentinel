use crate::{error::CryptoError, key_derivation_trait::KeyDerivationFunction};

/// Simple BLAKE3-based key derivation.
/// This is a placeholder implementation using BLAKE3 directly.
/// In production, this should be replaced with PBKDF2, Argon2, or scrypt.
///
/// WARNING: This implementation is NOT suitable for production use.
/// It provides no work factor and minimal protection against brute force attacks.
/// Use only for development/testing purposes.
pub struct Blake3KeyDerivation;

impl KeyDerivationFunction for Blake3KeyDerivation {
    fn derive_key_from_passphrase(passphrase: &str) -> Result<[u8; 32], CryptoError> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(passphrase.as_bytes());
        let hash = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash.as_bytes()[..32]);
        Ok(key)
    }
}

impl crate::key_derivation_trait::private::Sealed for Blake3KeyDerivation {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key() {
        let key = Blake3KeyDerivation::derive_key_from_passphrase("test").unwrap();
        assert_eq!(key.len(), 32);

        // Same passphrase should give same key
        let key2 = Blake3KeyDerivation::derive_key_from_passphrase("test").unwrap();
        assert_eq!(key, key2);

        // Different passphrase should give different key
        let key3 = Blake3KeyDerivation::derive_key_from_passphrase("different").unwrap();
        assert_ne!(key, key3);
    }
}