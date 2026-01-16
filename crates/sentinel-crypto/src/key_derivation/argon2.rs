use argon2::{Argon2, Params};
use rand::RngCore;
use tracing::{debug, trace};

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
/// - Salt: Randomly generated 32 bytes
pub struct Argon2KeyDerivation;

impl KeyDerivationFunction for Argon2KeyDerivation {
    fn derive_key_from_passphrase(passphrase: &str) -> Result<(Vec<u8>, [u8; 32]), CryptoError> {
        trace!("Deriving key from passphrase with Argon2 (generating salt)");
        let mut salt = [0u8; 32];
        rand::rng().fill_bytes(&mut salt);
        let salt_vec = salt.to_vec();

        let mut output_key_material = [0u8; 32];

        // Use recommended parameters for key derivation
        let params = Params::new(65536, 3, 1, Some(32))
            .map_err(|_| CryptoError::KeyDerivation(KeyDerivationError::DerivationFailed))?;

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        argon2
            .hash_password_into(passphrase.as_bytes(), &salt, &mut output_key_material)
            .map_err(|_| CryptoError::KeyDerivation(KeyDerivationError::DerivationFailed))?;

        debug!("Argon2 key derivation completed successfully");
        Ok((salt_vec, output_key_material))
    }

    fn derive_key_from_passphrase_with_salt(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], CryptoError> {
        trace!("Deriving key from passphrase with Argon2 (using provided salt)");
        let mut output_key_material = [0u8; 32];

        // Use recommended parameters for key derivation
        let params = Params::new(65536, 3, 1, Some(32))
            .map_err(|_| CryptoError::KeyDerivation(KeyDerivationError::DerivationFailed))?;

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        argon2
            .hash_password_into(passphrase.as_bytes(), salt, &mut output_key_material)
            .map_err(|_| CryptoError::KeyDerivation(KeyDerivationError::DerivationFailed))?;

        debug!("Argon2 key derivation with salt completed successfully");
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
        let (salt1, key1) = Argon2KeyDerivation::derive_key_from_passphrase(passphrase).unwrap();
        assert_eq!(key1.len(), 32);
        assert_eq!(salt1.len(), 32);

        // Same passphrase with different random salt should produce different keys
        let (salt2, key2) = Argon2KeyDerivation::derive_key_from_passphrase(passphrase).unwrap();
        assert_ne!(salt1, salt2);
        assert_ne!(key1, key2);

        // Same passphrase with same salt should produce same key
        let key1_again = Argon2KeyDerivation::derive_key_from_passphrase_with_salt(passphrase, &salt1).unwrap();
        assert_eq!(key1, key1_again);

        // Different passphrase should produce different key
        let (_salt3, key3) = Argon2KeyDerivation::derive_key_from_passphrase("different").unwrap();
        assert_ne!(key1, key3);
    }
}
