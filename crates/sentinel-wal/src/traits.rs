//! Traits for WAL operations that require document access

use serde_json::Value;

use crate::{EntryType, Result};

/// Operations required for WAL recovery and verification
#[async_trait::async_trait]
pub trait WalDocumentOps: Send + Sync {
    /// Get a document by ID
    async fn get_document(&self, id: &str) -> Result<Option<Value>>;

    /// Apply a WAL operation to a document
    async fn apply_operation(&self, entry_type: &EntryType, id: &str, data: Option<Value>) -> Result<()>;

    /// Set recovery mode (skip WAL logging during recovery)
    fn set_recovery_mode(&self, mode: bool) {
        // Default implementation does nothing
        let _ = mode;
    }
}
