use crate::error::CryptoError;

/// Core trait for key derivation functions used in sentinel-crypto.
/// This trait abstracts key derivation operations to allow easy switching between
/// different KDF algorithms while maintaining a consistent interface.
///
/// Design choice: Trait-based design enables compile-time algorithm selection
/// and allows for future extensions (e.g., Argon2, scrypt) without changing
/// the API. The trait is sealed to prevent external implementations that
/// might not meet security requirements.
pub trait KeyDerivationFunction: private::Sealed {
    /// Derives a 32-byte key from a passphrase using a randomly generated salt.
    /// Returns the salt and the derived key.
    /// This should use a proper key derivation function with appropriate parameters.
    ///
    /// # Arguments
    /// * `passphrase` - The passphrase to derive the key from
    ///
    /// # Returns
    /// A tuple of (salt, key) where salt is the randomly generated salt and key is the 32-byte
    /// derived key
    ///
    /// # Errors
    /// Returns `CryptoError::KeyDerivation` if derivation fails
    fn derive_key_from_passphrase(passphrase: &str) -> Result<(Vec<u8>, [u8; 32]), CryptoError>;

    /// Derives a 32-byte key from a passphrase using the provided salt.
    /// This is used when the salt has been previously generated and stored.
    ///
    /// # Arguments
    /// * `passphrase` - The passphrase to derive the key from
    /// * `salt` - The salt to use for derivation
    ///
    /// # Returns
    /// A 32-byte key suitable for encryption
    ///
    /// # Errors
    /// Returns `CryptoError::KeyDerivation` if derivation fails
    fn derive_key_from_passphrase_with_salt(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], CryptoError>;
}

// Sealing the trait to prevent external implementations
pub(crate) mod private {
    pub trait Sealed {}
}
