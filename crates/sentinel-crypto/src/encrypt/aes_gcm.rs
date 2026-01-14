use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use rand::RngCore;

use crate::{encrypt_trait::EncryptionAlgorithm, error::CryptoError};

/// AES-256-GCM encryption implementation.
/// Uses AES-256 in GCM mode for authenticated encryption, providing both
/// confidentiality and integrity. Generates random nonces for each encryption.
///
/// Design choice: AES-GCM was chosen for its security, performance, and
/// authenticated encryption properties. It's a rustcrypto crate, preferred
/// over ring implementations for consistency.
pub struct Aes256GcmEncryptor;

impl EncryptionAlgorithm for Aes256GcmEncryptor {
    fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<String, CryptoError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, data).map_err(|_| CryptoError::Encryption)?;
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(hex::encode(result))
    }

    fn decrypt_data(encrypted_data: &str, key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
        let data = hex::decode(encrypted_data).map_err(|_| CryptoError::Decryption)?;
        if data.len() < 12 {
            return Err(CryptoError::Decryption);
        }
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Nonce::from_slice(nonce_bytes);
        cipher.decrypt(nonce, ciphertext).map_err(|_| CryptoError::Decryption)
    }
}

impl crate::encrypt_trait::private::Sealed for Aes256GcmEncryptor {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0u8; 32];
        let data = b"Hello, world!";
        let encrypted = Aes256GcmEncryptor::encrypt_data(data, &key).unwrap();
        let decrypted = Aes256GcmEncryptor::decrypt_data(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }
}