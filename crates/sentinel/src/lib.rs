pub mod collection;
pub mod document;
pub mod error;
pub mod query;
pub mod store;
pub mod validation;

pub use collection::Collection;
pub use document::Document;
pub use error::{Result, SentinelError};
pub use query::{Filter, Operator, Query, QueryBuilder, QueryResult, SortOrder};
pub use sentinel_crypto::crypto_config::*;
pub use store::Store;

/// The current version of the Sentinel metadata format.
pub const META_SENTINEL_VERSION: u32 = 1;
