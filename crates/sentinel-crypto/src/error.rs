/// Comprehensive error type for all sentinel-crypto operations.
/// This enum wraps all possible errors that can occur during cryptographic operations,
/// providing a unified error handling interface. We use thiserror for ergonomic error
/// handling while ensuring all sensitive information is properly abstracted.
///
/// Design choice: Single error enum prevents error type proliferation and allows
/// for consistent error handling across the entire crypto crate. All errors are
/// wrapped to avoid leaking implementation details. Sub-enums (HashError, SignatureError,
/// KeyError) provide specific categorization while maintaining a flat top-level API.
///
/// Security consideration: Error messages are designed to not leak sensitive information
/// about keys, signatures, or internal state. All cryptographic failures are abstracted
/// to prevent side-channel attacks or information disclosure.
#[derive(thiserror::Error, Debug)]
pub enum CryptoError {
    /// Errors related to hashing operations
    #[error("Hashing error: {0}")]
    Hashing(#[from] HashError),

    /// Errors related to signature operations
    #[error("Signature error: {0}")]
    Signature(#[from] SignatureError),

    /// Errors related to key management
    #[error("Key management error: {0}")]
    KeyManagement(#[from] KeyError),

    /// Errors related to encryption operations
    #[error("Encryption error")]
    Encryption,

    /// Errors related to decryption operations
    #[error("Decryption error")]
    Decryption,

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Hex decoding errors
    #[error("Hex decoding error: {0}")]
    Hex(#[from] hex::FromHexError),

    /// Invalid signature length
    #[error("Invalid signature length")]
    InvalidSignatureLength,

    /// Invalid key length
    #[error("Invalid key length")]
    InvalidKeyLength,

    /// Verification failed
    #[error("Verification failed")]
    VerificationFailed,
}

/// Specific errors for hashing operations
#[derive(thiserror::Error, Debug)]
pub enum HashError {
    /// JSON serialization failed during hashing
    #[error("JSON serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Specific errors for signature operations
#[derive(thiserror::Error, Debug)]
pub enum SignatureError {
    /// Signature creation failed
    #[error("Signature creation failed")]
    SigningFailed,

    /// Signature verification failed
    #[error("Signature verification failed")]
    VerificationFailed,

    /// Invalid signature format
    #[error("Invalid signature format")]
    InvalidFormat,
}

/// Specific errors for key management operations
#[derive(thiserror::Error, Debug)]
pub enum KeyError {
    /// Key generation failed
    #[error("Key generation failed")]
    GenerationFailed,

    /// Key import failed
    #[error("Key import failed: {0}")]
    ImportFailed(String),

    /// Key export failed
    #[error("Key export failed")]
    ExportFailed,
}
