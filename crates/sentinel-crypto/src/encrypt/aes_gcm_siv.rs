use aes_gcm_siv::{
    aead::{Aead, KeyInit},
    Aes256GcmSiv,
    Key,
    Nonce,
};
use rand::RngCore;

use crate::{encrypt_trait::EncryptionAlgorithm, error::CryptoError};

/// AES-256-GCM-SIV encryption implementation.
/// Uses AES-256 in GCM-SIV mode for authenticated encryption with nonce misuse resistance.
/// Provides both confidentiality and integrity with better nonce handling than standard GCM.
///
/// Design choice: GCM-SIV provides nonce misuse resistance, making it safer than standard GCM
/// for applications where nonce uniqueness cannot be guaranteed. It's a rustcrypto crate,
/// preferred for consistency.
pub struct Aes256GcmSivEncryptor;

impl EncryptionAlgorithm for Aes256GcmSivEncryptor {
    fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<String, CryptoError> {
        let cipher = Aes256GcmSiv::new(Key::<Aes256GcmSiv>::from_slice(key));
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|_| CryptoError::Encryption)?;
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
        let cipher = Aes256GcmSiv::new(Key::<Aes256GcmSiv>::from_slice(key));
        let nonce = Nonce::from_slice(nonce_bytes);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::Decryption)
    }
}

impl crate::encrypt_trait::private::Sealed for Aes256GcmSivEncryptor {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0u8; 32];
        let data = b"Hello, world!";
        let encrypted = Aes256GcmSivEncryptor::encrypt_data(data, &key).unwrap();
        let decrypted = Aes256GcmSivEncryptor::decrypt_data(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_decrypt_invalid_length() {
        let key = [0u8; 32];
        // Short data that decodes to less than 12 bytes
        let short_hex = hex::encode(&[0u8; 10]); // 20 hex chars, 10 bytes
        let result = Aes256GcmSivEncryptor::decrypt_data(&short_hex, &key);
        assert!(result.is_err());
    }
}
