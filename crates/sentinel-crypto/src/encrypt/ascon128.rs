use ascon_aead::{
    aead::{Aead, KeyInit},
    Ascon128,
    Key,
    Nonce,
};
use rand::RngCore;

use crate::{encrypt_trait::EncryptionAlgorithm, error::CryptoError};

/// Ascon-128 encryption implementation.
/// Uses the Ascon authenticated encryption algorithm, designed for constrained environments.
/// Provides excellent performance on resource-limited devices while maintaining strong security.
///
/// Design choice: Ascon is specifically designed for IoT and constrained environments,
/// offering better performance than AES on systems without hardware acceleration.
/// It provides 128-bit security and is a finalist in the NIST lightweight cryptography competition.
pub struct Ascon128Encryptor;

impl EncryptionAlgorithm for Ascon128Encryptor {
    fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<String, CryptoError> {
        // Ascon128 uses 16-byte keys, so we take the first 16 bytes of the 32-byte key
        let key_16: &[u8; 16] = key[.. 16].try_into().unwrap();
        let cipher = Ascon128::new(Key::<Ascon128>::from_slice(key_16));
        let mut nonce_bytes = [0u8; 16]; // Ascon uses 128-bit nonce
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::<Ascon128>::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|_| CryptoError::Encryption)?;
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(hex::encode(result))
    }

    fn decrypt_data(encrypted_data: &str, key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
        let data = hex::decode(encrypted_data).map_err(|_| CryptoError::Decryption)?;
        if data.len() < 16 {
            return Err(CryptoError::Decryption);
        }
        let (nonce_bytes, ciphertext) = data.split_at(16);
        // Ascon128 uses 16-byte keys, so we take the first 16 bytes of the 32-byte key
        let key_16: &[u8; 16] = key[.. 16].try_into().unwrap();
        let cipher = Ascon128::new(Key::<Ascon128>::from_slice(key_16));
        let nonce = Nonce::<Ascon128>::from_slice(nonce_bytes);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::Decryption)
    }
}

impl crate::encrypt_trait::private::Sealed for Ascon128Encryptor {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0u8; 32];
        let data = b"Hello, world!";
        let encrypted = Ascon128Encryptor::encrypt_data(data, &key).unwrap();
        let decrypted = Ascon128Encryptor::decrypt_data(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }
}
