pub mod collection;
pub mod comparison;
pub mod document;
pub mod error;
pub mod filtering;
pub mod projection;
pub mod query;
pub mod store;
pub mod streaming;
pub mod validation;

pub use collection::Collection;
pub use document::Document;
pub use error::{Result, SentinelError};
// Re-export commonly used external crates for convenience
pub use futures::StreamExt;
pub use query::{Filter, Operator, Query, QueryBuilder, QueryResult, SortOrder};
pub use sentinel_crypto::crypto_config::*;
pub use serde_json::{json, Value};
pub use store::Store;

/// The current version of the Sentinel metadata format.
pub const META_SENTINEL_VERSION: u32 = 1;
