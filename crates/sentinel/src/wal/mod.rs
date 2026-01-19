//! WAL (Write-Ahead Logging) functionality for Sentinel DBMS.
//!
//! This module provides comprehensive WAL operations including configuration,
//! verification, recovery, and streaming capabilities. The WAL ensures data
//! durability and enables crash recovery for the filesystem-backed database.

pub mod ops;
pub mod recovery;
pub mod verification;
