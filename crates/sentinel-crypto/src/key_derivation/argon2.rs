use argon2::{Argon2, Params};

use crate::{
    error::{CryptoError, KeyDerivationError},
    key_derivation_trait::KeyDerivationFunction,
};

/// Argon2id key derivation implementation.
/// Uses Argon2id variant which provides the best of both Argon2i (resistance to side-channel
/// attacks) and Argon2d (resistance to time-memory trade-off attacks).
///
/// Design choice: Argon2id is the recommended variant for password hashing and key derivation.
/// It provides excellent security with configurable memory and time parameters.
///
/// Default parameters:
/// - Memory: 65536 KiB (64 MiB)
/// - Iterations: 3
/// - Parallelism: 1
pub struct Argon2KeyDerivation;

impl KeyDerivationFunction for Argon2KeyDerivation {
    fn derive_key_from_passphrase(passphrase: &str) -> Result<[u8; 32], CryptoError> {
        let mut output_key_material = [0u8; 32];

        // Use recommended parameters for key derivation
        let params = Params::new(65536, 3, 1, Some(32))
            .map_err(|_| CryptoError::KeyDerivation(KeyDerivationError::DerivationFailed))?;

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        argon2
            .hash_password_into(
                passphrase.as_bytes(),
                b"sentinel-salt",
                &mut output_key_material,
            )
            .map_err(|_| CryptoError::KeyDerivation(KeyDerivationError::DerivationFailed))?;

        Ok(output_key_material)
    }
}

impl crate::key_derivation_trait::private::Sealed for Argon2KeyDerivation {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_from_passphrase() {
        let passphrase = "test_passphrase";
        let key = Argon2KeyDerivation::derive_key_from_passphrase(passphrase).unwrap();
        assert_eq!(key.len(), 32);

        // Same passphrase should produce same key
        let key2 = Argon2KeyDerivation::derive_key_from_passphrase(passphrase).unwrap();
        assert_eq!(key, key2);

        // Different passphrase should produce different key
        let key3 = Argon2KeyDerivation::derive_key_from_passphrase("different").unwrap();
        assert_ne!(key, key3);
    }
}
