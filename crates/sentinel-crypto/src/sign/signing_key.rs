use ed25519_dalek::SigningKey;

/// Signing key management utilities
pub struct SigningKeyManager;

impl SigningKeyManager {
    /// Generate a new random signing key
    pub fn generate_key() -> SigningKey {
        let secret: [u8; 32] = rand::random();
        SigningKey::from_bytes(&secret)
    }

    /// Rotate key: generate new key and return both old and new
    /// For rotation, you might want to sign with both or something
    pub fn rotate_key(old_key: &SigningKey) -> (SigningKey, SigningKey) {
        let new_key = Self::generate_key();
        (old_key.clone(), new_key)
    }

    /// Export key as hex
    pub fn export_key(key: &SigningKey) -> String {
        hex::encode(key.to_bytes())
    }

    /// Import key from hex
    pub fn import_key(hex: &str) -> Result<SigningKey, Box<dyn std::error::Error>> {
        let bytes = hex::decode(hex)?;
        let array: [u8; 32] = bytes.as_slice().try_into().map_err(|_| "Invalid key length")?;
        Ok(SigningKey::from_bytes(&array))
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