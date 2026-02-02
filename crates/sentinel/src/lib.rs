/// Collection management module.
mod collection;
/// Comparison utilities module.
mod comparison;
/// Constants for special file and directory names.
mod constants;
/// Document handling module.
mod document;
/// Error types module.
mod error;
/// Event system module.
mod events;
/// Filtering utilities module.
mod filtering;
/// File locking system module.
pub mod locking;
/// Metadata management module.
mod metadata;
/// Projection utilities module.
mod projection;
/// Query building module.
mod query;
/// Store management module.
mod store;
/// Streaming utilities module.
mod streaming;
/// Validation utilities module.
mod validation;
/// Verification utilities module.
pub mod verification;
/// Write-Ahead Logging (WAL) operations module.
pub mod wal;

// Re-export commonly used external crates for convenience
pub use async_stream;
pub use futures;
// Re-export internal modules
pub use collection::Collection;
pub use constants::*;
pub use document::Document;
pub use error::{Result, SentinelError};
pub use query::{Aggregation, Filter, Operator, Query, QueryBuilder, QueryResult, SortOrder};
pub use sentinel_crypto::{
    crypto_config::*,
    derive_key_from_passphrase,
    encrypt_data,
    hash_data,
    sign_hash,
    verify_signature,
    CryptoError,
    EncryptionKeyManager,
    Signature,
    SigningKey,
    SigningKeyManager,
    VerifyingKey,
};
pub use store::Store;
pub use verification::{VerificationMode, VerificationOptions};
pub use metadata::{CollectionMetadata, MetadataVersion, StoreMetadata};
pub use sentinel_wal::{
    recover_from_wal_force,
    recover_from_wal_safe,
    verify_wal_consistency,
    CollectionWalConfig,
    CollectionWalConfigOverrides,
    CompressionAlgorithm,
    EntryType,
    LogEntry,
    StoreWalConfig,
    WalConfig,
    WalDocumentOps,
    WalError,
    WalFailureMode,
    WalFormat,
    WalManager,
    WalRecoveryFailure,
    WalRecoveryResult,
    WalVerificationIssue,
    WalVerificationResult,
};
pub use locking::{FileLockManager, LockGuard, LockManagerStats, LockStrategy};

/// The current version of the Sentinel metadata format.
pub const META_SENTINEL_VERSION: u32 = 2;
/// The current version of the Sentinel document format.
pub const DOCUMENT_SENTINEL_VERSION: u32 = 1;
