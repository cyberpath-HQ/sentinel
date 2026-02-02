//! # Lock Manager Statistics
//!
//! Provides statistics and metrics about the lock manager's current state.

use std::{collections::HashMap, time::Duration};

/// Statistics about the lock manager state.
#[derive(Debug, Clone)]
pub struct LockManagerStats {
    /// Number of paths currently locked
    pub active_locks:                usize,
    /// Total number of pending lock requests across all paths
    pub total_pending_requests:      usize,
    /// Number of paths with pending requests
    pub paths_with_pending_requests: usize,

    /// Current queue length per file path
    pub queue_lengths:       HashMap<String, usize>,
    /// Total time spent waiting in queues (all wait times summed)
    pub total_wait_time:     Duration,
    /// Total successful lock acquisitions
    pub lock_acquisitions:   u64,
    /// Total lock contention events (requests that waited for existing locks)
    pub lock_contentions:    u64,
    /// Total lock acquisitions that failed due to timeout
    pub failed_acquisitions: u64,
}

impl Default for LockManagerStats {
    fn default() -> Self { Self::new() }
}

impl LockManagerStats {
    /// Creates a new empty statistics struct with default values.
    ///
    /// # Returns
    ///
    /// A new `LockManagerStats` instance with all metrics initialized to zero or empty collections.
    pub fn new() -> Self {
        Self {
            active_locks:                0,
            total_pending_requests:      0,
            paths_with_pending_requests: 0,
            queue_lengths:               HashMap::new(),
            total_wait_time:             Duration::ZERO,
            lock_acquisitions:           0,
            lock_contentions:            0,
            failed_acquisitions:         0,
        }
    }

    /// Increments the number of active locks by 1.
    ///
    /// # Arguments
    ///
    /// * `path` - The path that was locked
    #[allow(
        clippy::arithmetic_side_effects,
        reason = "Lock stats mutate state; cannot be const"
    )]
    pub fn increment_active_lock(&mut self, path: String) {
        self.active_locks += 1;
        *self.queue_lengths.entry(path).or_insert(0) += 1;
    }

    /// Decrements the number of active locks by 1.
    ///
    /// # Arguments
    ///
    /// * `path` - The path that was unlocked
    #[allow(
        clippy::arithmetic_side_effects,
        reason = "Lock stats mutate state; cannot be const"
    )]
    pub fn decrement_active_lock(&mut self, path: &str) {
        if self.active_locks > 0 {
            self.active_locks -= 1;
        }
        if let Some(length) = self.queue_lengths.get_mut(path) {
            if *length > 0 {
                *length -= 1;
            }
            if *length == 0 {
                self.queue_lengths.remove(path);
            }
        }
    }

    /// Decrements the total pending requests counter.
    #[allow(
        clippy::arithmetic_side_effects,
        reason = "Lock stats mutate state; cannot be const"
    )]
    pub fn decrement_pending_requests(&mut self) {
        if self.total_pending_requests > 0 {
            self.total_pending_requests -= 1;
        }
    }

    /// Decrements the count of paths with pending requests.
    #[allow(
        clippy::arithmetic_side_effects,
        reason = "Lock stats mutate state; cannot be const"
    )]
    pub fn decrement_paths_with_pending(&mut self) {
        if self.paths_with_pending_requests > 0 {
            self.paths_with_pending_requests -= 1;
        }
    }

    /// Adds time spent waiting in queues.
    ///
    /// # Arguments
    ///
    /// * `duration` - The duration to add to total_wait_time
    #[allow(
        clippy::arithmetic_side_effects,
        reason = "Lock stats mutate state; cannot be const"
    )]
    pub fn add_wait_time(&mut self, duration: Duration) { self.total_wait_time += duration; }

    /// Increments the total lock acquisitions counter.
    #[allow(
        clippy::missing_const_for_fn,
        clippy::arithmetic_side_effects,
        reason = "const fn cannot mutate fields or perform arithmetic operations"
    )]
    pub fn increment_lock_acquisition(&mut self) { self.lock_acquisitions += 1; }

    /// Increments the total lock contention counter.
    #[allow(
        clippy::missing_const_for_fn,
        clippy::arithmetic_side_effects,
        reason = "const fn cannot mutate fields or perform arithmetic operations"
    )]
    pub fn increment_lock_contention(&mut self) { self.lock_contentions += 1; }

    /// Increments the total failed acquisitions counter.
    #[allow(
        clippy::missing_const_for_fn,
        clippy::arithmetic_side_effects,
        reason = "const fn cannot mutate fields or perform arithmetic operations"
    )]
    pub fn increment_failed_acquisition(&mut self) { self.failed_acquisitions += 1; }

    /// Gets the current queue length for a specific path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to check
    ///
    /// # Returns
    ///
    /// The current queue length, or 0 if the path is not in the queue.
    pub fn get_queue_length(&self, path: &str) -> usize { self.queue_lengths.get(path).copied().unwrap_or(0) }

    /// Gets the total wait time across all queues.
    ///
    /// # Returns
    ///
    /// The total duration spent waiting.
    pub const fn get_total_wait_time(&self) -> Duration { self.total_wait_time }

    /// Gets the average wait time per contention event.
    ///
    /// # Returns
    ///
    /// The average duration spent waiting per contention event.
    pub fn get_average_wait_time(&self) -> Duration {
        if self.lock_contentions > 0 {
            self.total_wait_time.div_f64(self.lock_contentions as f64)
        }
        else {
            Duration::ZERO
        }
    }

    /// Resets all metrics to zero, except for queue_lengths which is emptied.
    pub const fn reset(&mut self) {
        self.active_locks = 0;
        self.total_pending_requests = 0;
        self.paths_with_pending_requests = 0;
        self.total_wait_time = Duration::ZERO;
        self.lock_acquisitions = 0;
        self.lock_contentions = 0;
        self.failed_acquisitions = 0;
    }
}
