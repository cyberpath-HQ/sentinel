use chacha20poly1305::{
    aead::{Aead, KeyInit},
    Key,
    XChaCha20Poly1305,
    XNonce,
};
use rand::RngCore;

use crate::{encrypt_trait::EncryptionAlgorithm, error::CryptoError};

/// XChaCha20Poly1305 encryption implementation.
/// Uses the extended ChaCha20 nonce (XChaCha20) with Poly1305 for authenticated encryption.
/// Provides excellent performance and security, especially on systems without AES hardware
/// acceleration.
///
/// Design choice: XChaCha20Poly1305 is preferred over standard ChaCha20Poly1305 for its
/// larger nonce size (192 bits vs 96 bits), providing better nonce collision resistance.
/// It's a rustcrypto crate, preferred for consistency.
pub struct XChaCha20Poly1305Encryptor;

impl EncryptionAlgorithm for XChaCha20Poly1305Encryptor {
    fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<String, CryptoError> {
        let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
        let mut nonce_bytes = [0u8; 24]; // XChaCha20 uses 24-byte nonce
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|_| CryptoError::Encryption)?;
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(hex::encode(result))
    }

    fn decrypt_data(encrypted_data: &str, key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
        let data = hex::decode(encrypted_data).map_err(|_| CryptoError::Decryption)?;
        if data.len() < 24 {
            return Err(CryptoError::Decryption);
        }
        let (nonce_bytes, ciphertext) = data.split_at(24);
        let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = XNonce::from_slice(nonce_bytes);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::Decryption)
    }
}

impl crate::encrypt_trait::private::Sealed for XChaCha20Poly1305Encryptor {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0u8; 32];
        let data = b"Hello, world!";
        let encrypted = XChaCha20Poly1305Encryptor::encrypt_data(data, &key).unwrap();
        let decrypted = XChaCha20Poly1305Encryptor::decrypt_data(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }
}
