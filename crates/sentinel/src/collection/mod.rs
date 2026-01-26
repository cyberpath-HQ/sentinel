/// Collection aggregation operations.
pub mod aggregation;
/// Core collection implementation.
pub mod coll;
/// Collection operations.
pub mod operations;
/// Collection query operations.
pub mod query;
/// Collection streaming operations.
pub mod streaming;
#[cfg(test)]
/// Collection tests.
pub mod tests;
/// Collection verification operations.
pub mod verification;
/// Collection WAL operations.
pub mod wal;

pub use coll::*;
