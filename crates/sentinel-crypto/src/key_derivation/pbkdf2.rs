use pbkdf2::pbkdf2_hmac;
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
/// - Salt: "sentinel-pbkdf2-salt"
pub struct Pbkdf2KeyDerivation;

impl KeyDerivationFunction for Pbkdf2KeyDerivation {
    fn derive_key_from_passphrase(passphrase: &str) -> Result<[u8; 32], CryptoError> {
        let mut output_key_material = [0u8; 32];
        let salt = b"sentinel-pbkdf2-salt";

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
        let key = Pbkdf2KeyDerivation::derive_key_from_passphrase(passphrase).unwrap();
        assert_eq!(key.len(), 32);

        // Same passphrase should produce same key
        let key2 = Pbkdf2KeyDerivation::derive_key_from_passphrase(passphrase).unwrap();
        assert_eq!(key, key2);

        // Different passphrase should produce different key
        let key3 = Pbkdf2KeyDerivation::derive_key_from_passphrase("different").unwrap();
        assert_ne!(key, key3);
    }
}
