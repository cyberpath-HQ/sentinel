//! WAL recovery functionality.
//!
//! This module provides recovery of collections from WAL entries.
//! Unlike the previous flawed approach, this recovery:
//! 1. Only replays operations that haven't been applied yet
//! 2. Handles conflicts gracefully
//! 3. Is idempotent (can be run multiple times safely)

use sentinel_wal::{recover_from_wal_safe, recover_from_wal_force, WalRecoveryResult};

use crate::{Collection, Result};

impl Collection {
    /// Recover collection state from WAL entries
    ///
    /// This method replays WAL entries to restore the collection to its
    /// correct state. It only applies operations that haven't been applied yet
    /// and handles conflicts gracefully.
    pub async fn recover_from_wal_safe(&self) -> Result<WalRecoveryResult> {
        if let Some(wal) = &self.wal_manager {
            recover_from_wal_safe(wal, self).await.map_err(Into::into)
        } else {
            Ok(WalRecoveryResult {
                recovered_operations: 0,
                skipped_operations:   0,
                failed_operations:    0,
                failures:             vec![],
            })
        }
    }

    /// Recover collection from WAL with conflict resolution
    ///
    /// This is a more aggressive recovery that attempts to resolve conflicts
    /// by overwriting conflicting states.
    pub async fn recover_from_wal_force(&self) -> Result<WalRecoveryResult> {
        if let Some(wal) = &self.wal_manager {
            recover_from_wal_force(wal, self).await.map_err(Into::into)
        } else {
            Ok(WalRecoveryResult {
                recovered_operations: 0,
                skipped_operations:   0,
                failed_operations:    0,
                failures:             vec![],
            })
        }
    }
}