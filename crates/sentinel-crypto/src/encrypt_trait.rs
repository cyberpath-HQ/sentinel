use crate::error::CryptoError;

/// Core trait for encryption algorithms used in sentinel-crypto.
/// This trait abstracts encryption operations to allow easy switching between
/// different encryption algorithms while maintaining a consistent interface.
///
/// Design choice: Trait-based design enables compile-time algorithm selection
/// and allows for future extensions (e.g., ChaCha20-Poly1305, AES-GCM-SIV) without changing
/// the API. The trait is sealed to prevent external implementations that
/// might not meet security requirements.
pub trait EncryptionAlgorithm: private::Sealed {
    /// Encrypts the given data using the provided key.
    /// Returns a hex-encoded string containing nonce + ciphertext.
    ///
    /// # Arguments
    /// * `data` - The data to encrypt
    /// * `key` - The encryption key
    ///
    /// # Returns
    /// A hex-encoded string with nonce + ciphertext
    ///
    /// # Errors
    /// Returns `CryptoError::Encryption` if encryption fails
    fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<String, CryptoError>;

    /// Decrypts the given encrypted data using the provided key.
    /// Expects the input to be a hex-encoded string with nonce + ciphertext.
    ///
    /// # Arguments
    /// * `encrypted_data` - The hex-encoded nonce + ciphertext
    /// * `key` - The decryption key
    ///
    /// # Returns
    /// The decrypted data
    ///
    /// # Errors
    /// Returns `CryptoError::Decryption` if decryption fails
    fn decrypt_data(encrypted_data: &str, key: &[u8; 32]) -> Result<Vec<u8>, CryptoError>;
}

// Sealing the trait to prevent external implementations
pub(crate) mod private {
    pub trait Sealed {}
}
