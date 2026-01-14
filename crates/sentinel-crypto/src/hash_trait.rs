use crate::error::CryptoError;
use serde_json::Value;

/// Core trait for hash functions used in sentinel-crypto.
/// This trait abstracts hashing operations to allow easy switching between
/// different hash algorithms while maintaining a consistent interface.
///
/// Design choice: Trait-based design enables compile-time algorithm selection
/// and allows for future extensions (e.g., SHA-256, SHA-3) without changing
/// the API. The trait is sealed to prevent external implementations that
/// might not meet security requirements.
pub trait HashFunction: private::Sealed {
    /// Computes a cryptographic hash of the given JSON data.
    /// The data is canonicalized via JSON serialization before hashing to
    /// ensure deterministic results.
    ///
    /// # Arguments
    /// * `data` - The JSON value to hash
    ///
    /// # Returns
    /// A hex-encoded string representing the hash digest
    ///
    /// # Errors
    /// Returns `CryptoError::Hashing` if JSON serialization fails
    fn hash_data(data: &Value) -> Result<String, CryptoError>;
}

// Sealing the trait to prevent external implementations
pub(crate) mod private {
    pub trait Sealed {}
}