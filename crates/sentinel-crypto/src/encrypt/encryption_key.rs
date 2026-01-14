use crate::error::CryptoError;

/// Encryption key management utilities
pub struct EncryptionKeyManager;

impl EncryptionKeyManager {
    /// Generate a new random 256-bit encryption key
    pub fn generate_key() -> [u8; 32] { rand::random() }

    /// Rotate key: generate new key and return both old and new
    /// For rotation, you might want to re-encrypt data with the new key
    pub fn rotate_key(_old_key: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
        let new_key = Self::generate_key();
        (*_old_key, new_key)
    }

    /// Export key as hex
    pub fn export_key(key: &[u8; 32]) -> String { hex::encode(key) }

    /// Import key from hex
    pub fn import_key(hex: &str) -> Result<[u8; 32], CryptoError> {
        let bytes = hex::decode(hex).map_err(CryptoError::Hex)?;
        let array: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        Ok(array)
    }

    /// Generate a key from a passphrase using the default KDF
    pub fn derive_key_from_passphrase(passphrase: &str) -> Result<(Vec<u8>, [u8; 32]), CryptoError> {
        crate::derive_key_from_passphrase(passphrase)
    }

    /// Generate a key from a passphrase using the provided salt
    pub fn derive_key_from_passphrase_with_salt(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], CryptoError> {
        crate::derive_key_from_passphrase_with_salt(passphrase, salt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        let key = EncryptionKeyManager::generate_key();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_export_import_key() {
        let key = EncryptionKeyManager::generate_key();
        let hex = EncryptionKeyManager::export_key(&key);
        let imported = EncryptionKeyManager::import_key(&hex).unwrap();
        assert_eq!(key, imported);
    }

    #[test]
    fn test_rotate_key() {
        let old_key = EncryptionKeyManager::generate_key();
        let (returned_old, new_key) = EncryptionKeyManager::rotate_key(&old_key);
        assert_eq!(old_key, returned_old);
        assert_ne!(old_key, new_key);
    }

    #[test]
    fn test_derive_key() {
        let (salt1, key1) = EncryptionKeyManager::derive_key_from_passphrase("test").unwrap();
        assert_eq!(key1.len(), 32);
        assert_eq!(salt1.len(), 32);

        // Same passphrase with same salt should give same key
        let key1_again = EncryptionKeyManager::derive_key_from_passphrase_with_salt("test", &salt1).unwrap();
        assert_eq!(key1, key1_again);

        // Different passphrase should give different key
        let (_salt2, key2) = EncryptionKeyManager::derive_key_from_passphrase("different").unwrap();
        assert_ne!(key1, key2);
    }
}
