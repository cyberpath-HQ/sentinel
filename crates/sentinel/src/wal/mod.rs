//! WAL (Write-Ahead Logging) functionality for Sentinel DBMS.
//!
//! This module provides comprehensive WAL operations including configuration,
//! verification, recovery, and streaming capabilities. The WAL ensures data
//! durability and enables crash recovery for the filesystem-backed database.

pub mod config;
pub mod ops;
pub mod recovery;
pub mod verification;

// Re-export main types for convenience
pub use config::{CollectionWalConfig, StoreWalConfig, WalFailureMode};
pub use ops::{CollectionWalOps, StoreWalOps};
pub use verification::{WalVerificationIssue, WalVerificationResult};
pub use recovery::WalRecoveryResult;
