mod collection;
mod comparison;
mod document;
mod error;
mod filtering;
mod projection;
mod query;
mod store;
mod streaming;
mod validation;

pub use collection::Collection;
pub use document::Document;
pub use error::{Result, SentinelError};
pub use query::{Filter, Operator, Query, QueryBuilder, QueryResult, SortOrder};
pub use sentinel_crypto::crypto_config::*;
pub use store::Store;

// Re-export commonly used external crates for convenience
pub use async_stream;
pub use futures;

/// The current version of the Sentinel metadata format.
pub const META_SENTINEL_VERSION: u32 = 1;
