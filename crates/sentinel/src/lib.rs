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
/// Filtering utilities module.
mod filtering;
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
mod verification;
/// WAL (Write-Ahead Logging) operations module.
mod wal;

// Re-export commonly used external crates for convenience
pub use async_stream;
pub use futures;
// Re-export internal modules
pub use collection::Collection;
pub use constants::*;
pub use document::Document;
pub use error::{Result, SentinelError};
pub use query::{Aggregation, Filter, Operator, Query, QueryBuilder, QueryResult, SortOrder};
pub use sentinel_crypto::crypto_config::*;
pub use store::Store;
pub use verification::{VerificationMode, VerificationOptions};
pub use metadata::{CollectionMetadata, MetadataVersion, StoreMetadata};
pub use sentinel_wal::{CollectionWalConfig, StoreWalConfig, WalFailureMode};

/// The current version of the Sentinel metadata format.
pub const META_SENTINEL_VERSION: u32 = 1;
