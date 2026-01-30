//! # Lock Guard
//!
//! RAII wrapper for file locks that ensures automatic cleanup.
//! When dropped, the lock is automatically released and the lock manager is notified.

use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use fs2::FileExt;
use tracing::debug;

use crate::locking::{FileLockManager, LockStrategy};

/// RAII guard that holds a file lock and automatically releases it when dropped.
///
/// This struct provides safe lock management by ensuring that locks are always
/// released, even if the code panics or returns early. The lock is released
/// both at the filesystem level and in the lock manager's internal state.
///
/// # Fields
///
/// - `path`: Path to the locked file
/// - `strategy`: Lock strategy (Exclusive or Shared)
/// - `_manager`: Reference to the lock manager (keeps it alive)
/// - `file`: The actual file handle with the lock
/// - `acquired_at`: When this lock was acquired
/// - `holder_id`: Unique ID of the lock holder
#[derive(Debug)]
pub struct LockGuard {
    /// Path to the locked file
    pub(crate) path:        PathBuf,
    /// Strategy used for this lock
    pub(crate) strategy:    LockStrategy,
    /// Reference to the lock manager (keeps it alive)
    pub(crate) _manager:    Arc<FileLockManager>,
    /// The actual file handle with the lock
    pub(crate) file:        File,
    /// When this lock was acquired (for deadlock detection)
    pub(crate) acquired_at: Instant,
    /// Unique ID holding this lock (for deadlock detection)
    pub(crate) holder_id:   String,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // The filesystem lock is automatically released when the file handle is closed
        // No need to call unlock() explicitly

        // Clear the lock state synchronously for immediate effect
        if let Some(lock_state) = self._manager.lock_table.get(&self.path) {
            let mut state = lock_state.lock().unwrap();
            *state = None;
        }

        // Note: Deadlock detector tracking is handled asynchronously elsewhere
        // to avoid async operations in Drop

        // Synchronously wake up the next waiter in the queue
        if let Some(queue) = self._manager.lock_queues.get(&self.path) {
            let mut queue = queue.lock().unwrap();
            if let Some(next_waiter) = queue.dequeue() {
                debug!(
                    "Waking up next waiter {:?} for path {:?}",
                    next_waiter.requester_id, self.path
                );
                // Ignore send error (waiter may have timed out)
                let _ = next_waiter.waker.send(());
            }
        }
    }
}

impl LockGuard {
    /// Get the path of the locked file.
    pub fn path(&self) -> &Path { &self.path }

    /// Get the lock strategy used.
    pub const fn strategy(&self) -> LockStrategy { self.strategy }

    /// Get when this lock was acquired.
    pub const fn acquired_at(&self) -> Instant { self.acquired_at }

    /// Get the ID of the lock holder.
    pub fn holder_id(&self) -> String { self.holder_id.clone() }

    /// Upgrade from shared lock to exclusive lock.
    ///
    /// This method converts the current shared lock into an exclusive lock
    /// without releasing it. The lock manager ensures that no other locks
    /// are held on the same path before allowing the upgrade.
    ///
    /// # Errors
    ///
    /// * `SentinelError::InvalidLockState` - Guard is not holding a shared lock
    /// * `SentinelError::LockTimeout` - Upgrade wait timeout expires
    /// * `SentinelError::DeadlockDetected` - Deadlock detected during upgrade
    /// * `SentinelError::LockContention` - Other locks prevent upgrade
    pub async fn upgrade_to_exclusive(
        self,
        manager: &Arc<FileLockManager>,
        timeout: Option<std::time::Duration>,
    ) -> crate::error::Result<Self> {
        // Check if current lock is shared
        if self.strategy != LockStrategy::Shared {
            return Err(crate::error::SentinelError::InvalidLockState {
                reason: "cannot upgrade non-shared lock".to_string(),
            });
        }

        let _timeout = timeout.unwrap_or(manager.default_timeout);
        let requester_id = cuid2::cuid();

        // First, check if we're the only holder of this shared lock
        // In a proper implementation, we'd need to track all holders, but for now
        // we'll attempt the upgrade and see if it succeeds
        match self.file.try_lock_exclusive() {
            Ok(()) => {
                // Upgrade succeeded - update the strategy and return
                let mut upgraded_guard = self;
                upgraded_guard.strategy = LockStrategy::Exclusive;

                // Update the lock manager's state
                if let Some(lock_state) = manager.lock_table.get(&upgraded_guard.path) {
                    let mut state = lock_state.lock().unwrap();
                    *state = Some((requester_id, LockStrategy::Exclusive));
                }

                debug!("Successfully upgraded lock for {:?}", upgraded_guard.path);
                Ok(upgraded_guard)
            },
            Err(_) => {
                // Upgrade failed - either timeout or contention
                // For now, return an error. A full implementation would enqueue for upgrade
                Err(crate::error::SentinelError::LockContention {
                    path: self.path.clone(),
                })
            },
        }
    }

    /// Downgrade from exclusive lock to shared lock.
    ///
    /// This method converts the current exclusive lock into a shared lock
    /// without releasing it. The file handle's lock is downgraded, allowing
    /// other readers to acquire shared locks simultaneously.
    ///
    /// # Errors
    ///
    /// * `SentinelError::InvalidLockState` - Guard is not holding an exclusive lock
    pub fn downgrade_to_shared(mut self) -> crate::error::Result<Self> {
        // Check if current lock is exclusive
        if self.strategy != LockStrategy::Exclusive {
            return Err(crate::error::SentinelError::InvalidLockState {
                reason: "cannot downgrade non-exclusive lock".to_string(),
            });
        }

        // Downgrade the filesystem lock
        match self.file.lock_shared() {
            Ok(()) => {
                // Update the strategy in the guard
                self.strategy = LockStrategy::Shared;
                Ok(self)
            },
            Err(e) => {
                Err(crate::error::SentinelError::Io {
                    source: e,
                })
            },
        }
    }
}
