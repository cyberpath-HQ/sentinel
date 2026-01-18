use ed25519_dalek::SigningKey;
use tracing::{debug, trace};
use zeroize::Zeroize;

use crate::error::CryptoError;

/// Signing key management utilities
pub struct SigningKeyManager;

impl SigningKeyManager {
    /// Generate a new random signing key
    pub fn generate_key() -> SigningKey {
        trace!("Generating new signing key");
        let mut secret: [u8; 32] = rand::random();
        let key = SigningKey::from_bytes(&secret);
        secret.zeroize();
        debug!("Signing key generated successfully");
        key
    }

    /// Rotate key: generate new key and return both old and new
    /// For rotation, you might want to sign with both or something
    pub fn rotate_key(old_key: &SigningKey) -> (SigningKey, SigningKey) {
        trace!("Rotating signing key");
        let new_key = Self::generate_key();
        debug!("Signing key rotated successfully");
        (old_key.clone(), new_key)
    }

    /// Export key as hex
    pub fn export_key(key: &SigningKey) -> String {
        trace!("Exporting signing key");
        let hex = hex::encode(key.to_bytes());
        debug!("Signing key exported");
        hex
    }

    /// Import key from hex
    pub fn import_key(hex: &str) -> Result<SigningKey, CryptoError> {
        trace!("Importing signing key from hex");
        let bytes = hex::decode(hex).map_err(CryptoError::Hex)?;
        let mut array: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        let key = SigningKey::from_bytes(&array);
        array.zeroize();
        debug!("Signing key imported successfully");
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        let key = SigningKeyManager::generate_key();
        assert_eq!(key.to_bytes().len(), 32);
    }

    #[test]
    fn test_export_import_key() {
        let key = SigningKeyManager::generate_key();
        let hex = SigningKeyManager::export_key(&key);
        let imported = SigningKeyManager::import_key(&hex).unwrap();
        assert_eq!(key.to_bytes(), imported.to_bytes());
    }

    #[test]
    fn test_rotate_key() {
        let old = SigningKeyManager::generate_key();
        let (old_rot, new) = SigningKeyManager::rotate_key(&old);
        assert_eq!(old.to_bytes(), old_rot.to_bytes());
        assert_ne!(old.to_bytes(), new.to_bytes());
    }
}
