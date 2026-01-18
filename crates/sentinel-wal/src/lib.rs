//! # Sentinel WAL (Write-Ahead Logging)
//!
//! This crate implements the Write-Ahead Logging (WAL) functionality for Cyberpath Sentinel.
//! WAL ensures durability and crash recovery by logging all changes before they are applied
//! to the filesystem.
//!
//! ## Architecture
//!
//! The WAL consists of log entries written to a binary file. Each entry contains:
//! - Entry type (1 byte)
//! - Transaction ID (variable length, cuid2 string)
//! - Collection name (variable length string)
//! - Document ID (variable length string)
//! - Data length (8 bytes)
//! - Data (variable length, JSON)
//! - CRC32 checksum (4 bytes)
//!
//! ## Features
//!
//! - Postcard serialization for efficiency and maintainability
//! - CRC32 checksums for integrity
//! - Asynchronous I/O operations
//! - Checkpoint mechanism for log compaction
//! - Crash recovery via log replay

pub mod entry;
pub mod error;
pub mod manager;
#[cfg(test)]
mod tests;

// Re-exports
pub use error::WalError;
pub use entry::{EntryType, LogEntry};
pub use manager::WalManager;

/// Result type for WAL operations
pub type Result<T> = std::result::Result<T, WalError>;
