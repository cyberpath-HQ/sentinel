pub mod collection;
pub mod document;
pub mod error;
pub mod store;
pub mod validation;

pub use collection::Collection;
pub use document::Document;
pub use error::{Result, SentinelError};
pub use store::Store;

/// The current version of the Sentinel metadata format.
pub const META_SENTINEL_VERSION: u32 = 1;
