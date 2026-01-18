//! Error types for WAL operations

use std::io;

/// Error types for WAL operations
#[derive(thiserror::Error, Debug)]
pub enum WalError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Invalid log entry: {0}")]
    InvalidEntry(String),
    #[error("Checksum mismatch")]
    ChecksumMismatch,
    #[error("File size limit exceeded")]
    FileSizeLimitExceeded,
    #[error("Record limit exceeded")]
    RecordLimitExceeded,
}
