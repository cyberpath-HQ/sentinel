use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;

use crate::{error::CryptoError, key_derivation_trait::KeyDerivationFunction};

/// PBKDF2 key derivation implementation.
/// Uses PBKDF2 with HMAC-SHA256 for key derivation, suitable for constrained environments.
/// Provides good security with lower memory requirements compared to Argon2.
///
/// Design choice: PBKDF2 is widely supported and provides good security for constrained
/// environments where Argon2's memory requirements might be too high.
/// It uses HMAC-SHA256 as the underlying PRF.
///
/// Default parameters:
/// - Iterations: 100,000
/// - Salt: Randomly generated 32 bytes
pub struct Pbkdf2KeyDerivation;

impl KeyDerivationFunction for Pbkdf2KeyDerivation {
    fn derive_key_from_passphrase(passphrase: &str) -> Result<(Vec<u8>, [u8; 32]), CryptoError> {
        let mut salt = [0u8; 32];
        rand::rng().fill_bytes(&mut salt);
        let salt_vec = salt.to_vec();

        let mut output_key_material = [0u8; 32];

        // Use 100,000 iterations for good security in constrained environments
        pbkdf2_hmac::<Sha256>(
            passphrase.as_bytes(),
            &salt,
            100_000,
            &mut output_key_material,
        );

        Ok((salt_vec, output_key_material))
    }

    fn derive_key_from_passphrase_with_salt(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], CryptoError> {
        let mut output_key_material = [0u8; 32];

        // Use 100,000 iterations for good security in constrained environments
        pbkdf2_hmac::<Sha256>(
            passphrase.as_bytes(),
            salt,
            100_000,
            &mut output_key_material,
        );

        Ok(output_key_material)
    }
}

impl crate::key_derivation_trait::private::Sealed for Pbkdf2KeyDerivation {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_from_passphrase() {
        let passphrase = "test_passphrase";
        let (salt1, key1) = Pbkdf2KeyDerivation::derive_key_from_passphrase(passphrase).unwrap();
        assert_eq!(key1.len(), 32);
        assert_eq!(salt1.len(), 32);

        // Same passphrase with different random salt should produce different keys
        let (salt2, key2) = Pbkdf2KeyDerivation::derive_key_from_passphrase(passphrase).unwrap();
        assert_ne!(salt1, salt2);
        assert_ne!(key1, key2);

        // Same passphrase with same salt should produce same key
        let key1_again = Pbkdf2KeyDerivation::derive_key_from_passphrase_with_salt(passphrase, &salt1).unwrap();
        assert_eq!(key1, key1_again);

        // Different passphrase should produce different key
        let (_salt3, key3) = Pbkdf2KeyDerivation::derive_key_from_passphrase("different").unwrap();
        assert_ne!(key1, key3);
    }
}
