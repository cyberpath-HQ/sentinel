use thiserror::Error;

/// Sentinel-wide error type for the document DBMS.
///
/// This error type encompasses all possible errors that can occur within
/// the Sentinel system, providing structured error handling and meaningful
/// error messages for different failure scenarios.
#[derive(Error, Debug)]
pub enum SentinelError {
    /// I/O operations failed (file system, network, etc.)
    #[error("I/O error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    /// JSON serialization/deserialization failed
    #[error("JSON error: {source}")]
    Json {
        #[from]
        source: serde_json::Error,
    },

    /// Document not found in collection
    #[error("Document '{id}' not found in collection '{collection}'")]
    DocumentNotFound {
        id: String,
        collection: String,
    },

    /// Collection not found in store
    #[error("Collection '{name}' not found in store")]
    CollectionNotFound {
        name: String,
    },

    /// Document already exists (for operations that require uniqueness)
    #[error("Document '{id}' already exists in collection '{collection}'")]
    DocumentAlreadyExists {
        id: String,
        collection: String,
    },

    /// Invalid document ID format
    #[error("Invalid document ID: {id}")]
    InvalidDocumentId {
        id: String,
    },

    /// Invalid collection name format
    #[error("Invalid collection name: {name}")]
    InvalidCollectionName {
        name: String,
    },

    /// Store is corrupted or in an invalid state
    #[error("Store corruption detected: {reason}")]
    StoreCorruption {
        reason: String,
    },

    /// Transaction failed
    #[error("Transaction failed: {reason}")]
    TransactionFailed {
        reason: String,
    },

    /// Lock acquisition failed
    #[error("Lock acquisition failed: {reason}")]
    LockFailed {
        reason: String,
    },

    /// Encryption/decryption operation failed
    #[error("Cryptographic operation failed: {operation}")]
    CryptoFailed {
        operation: String,
    },

    /// Configuration error
    #[error("Configuration error: {message}")]
    ConfigError {
        message: String,
    },

    /// Generic error for unexpected conditions
    #[error("Internal error: {message}")]
    Internal {
        message: String,
    },
}

/// Result type alias for Sentinel operations.
pub type Result<T> = std::result::Result<T, SentinelError>;
