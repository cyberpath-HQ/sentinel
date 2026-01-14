use crate::error::CryptoError;

/// Core trait for signature algorithms used in sentinel-crypto.
/// This trait abstracts digital signature operations to allow easy switching
/// between different signature schemes while maintaining a consistent interface.
///
/// Design choice: Associated types for key types ensure type safety at compile-time.
/// The trait is sealed to prevent insecure external implementations. All operations
/// return our unified CryptoError for consistent error handling.
pub trait SignatureAlgorithm: private::Sealed {
    /// The type of the signing key
    type SigningKey;
    /// The type of the verifying key
    type VerifyingKey;
    /// The type of the signature
    type Signature;

    /// Signs the given hash using the provided private key.
    /// Returns a hex-encoded signature string.
    ///
    /// # Arguments
    /// * `hash` - The hash to sign (as a string)
    /// * `private_key` - The signing key
    ///
    /// # Returns
    /// A hex-encoded signature string
    ///
    /// # Errors
    /// Returns `CryptoError::Signature` if signing fails
    fn sign_hash(hash: &str, private_key: &Self::SigningKey) -> Result<String, CryptoError>;

    /// Verifies a signature against the given hash using the provided public key.
    ///
    /// # Arguments
    /// * `hash` - The original hash (as a string)
    /// * `signature` - The hex-encoded signature to verify
    /// * `public_key` - The verifying key
    ///
    /// # Returns
    /// `true` if verification succeeds, `false` otherwise
    ///
    /// # Errors
    /// Returns `CryptoError` if verification process fails
    fn verify_signature(
        hash: &str,
        signature: &str,
        public_key: &Self::VerifyingKey,
    ) -> Result<bool, CryptoError>;
}

// Sealing the trait to prevent external implementations
pub(crate) mod private {
    pub trait Sealed {}
}