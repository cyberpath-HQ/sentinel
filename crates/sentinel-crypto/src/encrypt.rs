use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use rand::RngCore;

use crate::error::CryptoError;

/// Encrypts the given data using AES-256-GCM with the provided key.
/// Generates a random nonce and returns the nonce + ciphertext.
///
/// # Arguments
/// * `data` - The data to encrypt
/// * `key` - The 32-byte encryption key
///
/// # Returns
/// The nonce (12 bytes) + ciphertext as a hex string
pub fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<String, CryptoError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, data).map_err(|_| CryptoError::Encryption)?;
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(hex::encode(result))
}

/// Decrypts the given data using AES-256-GCM.
/// Expects the input to be nonce (12 bytes) + ciphertext as hex string.
///
/// # Arguments
/// * `encrypted_data` - The hex-encoded nonce + ciphertext
/// * `key` - The 32-byte decryption key
///
/// # Returns
/// The decrypted data
pub fn decrypt_data(encrypted_data: &str, key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    let data = hex::decode(encrypted_data).map_err(|_| CryptoError::Decryption)?;
    if data.len() < 12 {
        return Err(CryptoError::Decryption);
    }
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher.decrypt(nonce, ciphertext).map_err(|_| CryptoError::Decryption)
}

/// Derives a 32-byte key from a passphrase using a simple hash.
/// Note: In production, use PBKDF2 or Argon2.
pub fn derive_key_from_passphrase(passphrase: &str) -> [u8; 32] {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(passphrase.as_bytes());
    let hash = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash.as_bytes()[..32]);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0u8; 32];
        let data = b"Hello, world!";
        let encrypted = encrypt_data(data, &key).unwrap();
        let decrypted = decrypt_data(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_derive_key() {
        let key = derive_key_from_passphrase("test");
        assert_eq!(key.len(), 32);
    }
}