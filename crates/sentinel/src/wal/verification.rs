//! WAL verification functionality.
//!
//! This module provides verification of WAL consistency and data integrity.
//! Unlike the previous flawed approach, this verifies:
//! 1. WAL internal consistency (operations are valid sequences)
//! 2. Final WAL state matches current disk state
//! 3. No corrupted or invalid entries exist

use sentinel_wal::{verify_wal_consistency, WalVerificationResult};

use crate::{Collection, Result};

impl Collection {
    /// Verify WAL consistency and final state against disk
    ///
    /// This method:
    /// 1. Replays all WAL entries to compute final expected states
    /// 2. Compares final WAL states with actual disk states
    /// 3. Checks for WAL internal consistency
    pub async fn verify_wal_consistency(&self) -> Result<WalVerificationResult> {
        if let Some(wal) = &self.wal_manager {
            verify_wal_consistency(wal, self).await.map_err(Into::into)
        } else {
            Ok(WalVerificationResult {
                issues: vec![],
                passed: true,
                entries_processed: 0,
                affected_documents: 0,
            })
        }
    }
}