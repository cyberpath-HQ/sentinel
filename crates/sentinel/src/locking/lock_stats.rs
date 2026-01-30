//! # Lock Manager Statistics
//!
//! Provides statistics and metrics about the lock manager's current state.

/// Statistics about the lock manager state.
#[derive(Debug, Clone)]
pub struct LockManagerStats {
    /// Number of paths currently locked
    pub active_locks:                usize,
    /// Total number of pending lock requests across all paths
    pub total_pending_requests:      usize,
    /// Number of paths with pending requests
    pub paths_with_pending_requests: usize,
}
