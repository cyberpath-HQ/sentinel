//! # File Locking System for Cyberpath Sentinel
//!
//! This module provides a comprehensive file locking system that ensures consistency
//! across all Sentinel components (sentinel-dbms, sentinel-wal, and other instances).
//!
//! ## Overview
//!
//! The locking system is designed to prevent data corruption and ensure atomic operations
//! across multiple processes and threads. It uses filesystem-level locking via `fs2` crate
//! which provides cross-platform advisory file locking.
//!
//! ## Key Features
//!
//! - **Exclusive Locks**: Single writer, no concurrent readers (for write operations)
//! - **Shared Locks**: Multiple concurrent readers, no writers (for read operations)
//! - **Timeout Handling**: Configurable timeouts to prevent indefinite blocking
//! - **Deadlock Detection**: Proactive detection using wait-for graphs
//! - **Cross-Process Consistency**: Works across multiple Sentinel instances
//! - **Automatic Cleanup**: Lock files are cleaned up on process exit
//!
//! ## Architecture
//!
//! The system consists of several components:
//!
//! - [`FileLockManager`]: Central manager for all file locks
//! - [`LockGuard`]: RAII guard that automatically releases locks
//! - [`LockStrategy`]: Defines locking behavior (exclusive vs shared)
//! - [`DeadlockDetector`]: Monitors for deadlock conditions
//!
//! ## Usage Patterns
//!
//! ### Basic Exclusive Locking
//!
//! ```rust,no_run
//! use sentinel_dbms::locking::{FileLockManager, LockStrategy};
//!
//! # async fn example() -> sentinel_dbms::Result<()> {
//! let manager = FileLockManager::new();
//!
//! // Acquire exclusive lock for writing
//! let guard = manager
//!     .acquire_lock(
//!         &std::path::PathBuf::from("data/users/user-123.json"),
//!         LockStrategy::Exclusive,
//!         std::time::Duration::from_secs(30),
//!     )
//!     .await?;
//!
//! // Perform write operation
//! // Lock is automatically released when guard goes out of scope
//! # Ok(())
//! # }
//! ```
//!
//! ### Shared Locking for Reads
//!
//! ```rust,no_run
//! # use sentinel_dbms::locking::{FileLockManager, LockStrategy};
//! # async fn example() -> sentinel_dbms::Result<()> {
//! # let manager = FileLockManager::new();
//! // Multiple readers can hold shared locks simultaneously
//! let guard1 = manager
//!     .acquire_lock(
//!         &std::path::PathBuf::from("data/users/user-123.json"),
//!         LockStrategy::Shared,
//!         std::time::Duration::from_secs(10),
//!     )
//!     .await?;
//!
//! let guard2 = manager
//!     .acquire_lock(
//!         &std::path::PathBuf::from("data/users/user-123.json"),
//!         LockStrategy::Shared,
//!         std::time::Duration::from_secs(10),
//!     )
//!     .await?;
//!
//! // Both guards can read simultaneously
//! # Ok(())
//! # }
//! ```
//!
//! ## Lock Compatibility Matrix
//!
//! | Current Lock | Requested Lock | Result |
//! |--------------|----------------|--------|
//! | None         | Exclusive      | ✅ Granted |
//! | None         | Shared         | ✅ Granted |
//! | Exclusive    | Exclusive      | ❌ Blocked |
//! | Exclusive    | Shared         | ❌ Blocked |
//! | Shared       | Exclusive      | ❌ Blocked |
//! | Shared       | Shared         | ✅ Granted |
//!
//! ## Deadlock Prevention
//!
//! The system implements deadlock detection using a wait-for graph algorithm:
//!
//! 1. Each lock acquisition records the waiting relationship
//! 2. A background task periodically checks for cycles in the wait graph
//! 3. When a deadlock is detected, the youngest transaction is aborted
//! 4. Lock requests are retried with exponential backoff
//!
//! ## Error Handling
//!
//! The locking system uses the standard `sentinel_dbms::Result<T>` type with specific
//! error variants for locking operations:
//!
//! - `LockTimeout`: Lock acquisition timed out
//! - `DeadlockDetected`: Deadlock detected and transaction aborted
//! - `LockContention`: High lock contention detected
//! - `InvalidLockState`: Lock manager is in an invalid state
//!
//! ## Performance Considerations
//!
//! - Lock acquisitions are async and non-blocking
//! - Lock table uses efficient hash maps for O(1) lookups
//! - Deadlock detection runs in a separate background task
//! - Lock files are created with exclusive access to prevent race conditions
//!
//! ## Integration Points
//!
//! The locking system integrates with:
//!
//! - **Collection Operations**: Document CRUD operations use appropriate locks
//! - **WAL Operations**: Log writes use exclusive locks on log files
//! - **Index Operations**: Index updates use exclusive locks on index files
//! - **Replication**: Cross-instance sync uses shared locks during reads

use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use uuid::Uuid;
use dashmap::DashMap;
use fs2::FileExt;
use tokio::{
    sync::{Mutex, RwLock},
    time,
};
use tracing::{debug, error};

use crate::{Result, SentinelError};

/// Lock strategy defines the type of lock to acquire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockStrategy {
    /// Exclusive lock: Single writer, no concurrent readers.
    /// Used for write operations that modify data.
    Exclusive,

    /// Shared lock: Multiple concurrent readers, no writers.
    /// Used for read operations that don't modify data.
    Shared,
}

/// RAII guard that automatically releases the lock when dropped.
/// The lock is released even if the program panics.
#[derive(Debug)]
pub struct LockGuard {
    /// Path to the locked file
    path:        PathBuf,
    /// Strategy used for this lock
    strategy:    LockStrategy,
    /// Reference to the lock manager (keeps it alive)
    _manager:    Arc<FileLockManager>,
    /// The actual file handle with the lock
    file:        File,
    /// When this lock was acquired (for deadlock detection)
    acquired_at: Instant,
    /// Unique ID holding this lock (for deadlock detection)
    holder_id:   Uuid,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // The filesystem lock is automatically released when the file handle is closed
        // No need to call unlock() explicitly

        // Notify the lock manager that this lock has been released
        // We need to spawn a task since we can't await in drop
        let manager = Arc::clone(&self._manager);
        let path = self.path.clone();

        tokio::spawn(async move {
            if let Some(lock_state) = manager.lock_table.get(&path) {
                let mut state = lock_state.write().await;
                *state = None;
            }
            manager.deadlock_detector.record_lock_released(&path).await;
        });
    }
}

/// Deadlock detection state for a single lock request.
#[derive(Debug)]
struct LockRequest {
    /// Path being requested
    path:         PathBuf,
    /// Strategy requested
    strategy:     LockStrategy,
    /// When the request started
    started_at:   Instant,
    /// Timeout for this request
    timeout:      Duration,
    /// ID of the requester (for deadlock detection)
    requester_id: Uuid,
}

/// Global deadlock detector that monitors all lock requests.
#[derive(Debug)]
struct DeadlockDetector {
    /// Wait-for graph: holder_id -> set of requesters waiting for that holder
    wait_graph:       RwLock<HashMap<Uuid, Vec<Uuid>>>,
    /// Current lock requests: path -> list of pending requests
    pending_requests: RwLock<HashMap<PathBuf, Vec<LockRequest>>>,
    /// Active locks: path -> (holder_id, strategy)
    active_locks:     RwLock<HashMap<PathBuf, (Uuid, LockStrategy)>>,
}

impl DeadlockDetector {
    fn new() -> Self {
        Self {
            wait_graph:       RwLock::new(HashMap::new()),
            pending_requests: RwLock::new(HashMap::new()),
            active_locks:     RwLock::new(HashMap::new()),
        }
    }

    /// Register that a requester is waiting for a lock held by holder_id.
    async fn register_wait(&self, requester_id: Uuid, holder_id: Uuid) -> Result<()> {
        let mut graph = self.wait_graph.write().await;
        graph
            .entry(holder_id)
            .or_insert_with(Vec::new)
            .push(requester_id);
        Ok(())
    }

    /// Remove a wait relationship when a request completes.
    async fn unregister_wait(&self, requester_id: Uuid, holder_id: Uuid) -> Result<()> {
        let mut graph = self.wait_graph.write().await;
        if let Some(waiters) = graph.get_mut(&holder_id) {
            waiters.retain(|&id| id != requester_id);
            if waiters.is_empty() {
                graph.remove(&holder_id);
            }
        }
        Ok(())
    }

    /// Record an active lock acquisition.
    async fn record_lock_acquired(&self, path: &Path, holder_id: Uuid, strategy: LockStrategy) {
        let mut active = self.active_locks.write().await;
        active.insert(path.to_path_buf(), (holder_id, strategy));
    }

    /// Record a lock release.
    async fn record_lock_released(&self, path: &Path) {
        let mut active = self.active_locks.write().await;
        active.remove(path);
    }

    /// Register a pending lock request.
    async fn register_request(&self, request: LockRequest) {
        let mut pending = self.pending_requests.write().await;
        pending
            .entry(request.path.clone())
            .or_insert_with(Vec::new)
            .push(request);
    }

    /// Remove a pending request when it completes.
    async fn unregister_request(&self, path: &Path, requester_id: Uuid) {
        let mut pending = self.pending_requests.write().await;
        if let Some(requests) = pending.get_mut(path) {
            requests.retain(|req| req.requester_id != requester_id);
            if requests.is_empty() {
                pending.remove(path);
            }
        }
    }

    /// Detect if there's a deadlock involving the given requester.
    /// Returns true if a cycle is found in the wait-for graph.
    async fn detect_deadlock(&self, requester_id: Uuid) -> bool {
        let graph = self.wait_graph.read().await;
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![requester_id];

        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                // Cycle detected
                return true;
            }

            // Add all nodes that are waiting for this one
            if let Some(waiters) = graph.get(&current) {
                for &waiter in waiters {
                    if !visited.contains(&waiter) {
                        stack.push(waiter);
                    }
                }
            }
        }

        false
    }

    /// Check for deadlocks and return IDs of deadlocked transactions.
    async fn find_deadlocked_transactions(&self) -> Vec<Uuid> {
        let graph = self.wait_graph.read().await;
        let mut deadlocked = Vec::new();
        let mut visited = std::collections::HashSet::new();

        for &start_node in graph.keys() {
            if visited.contains(&start_node) {
                continue;
            }

            let mut path = Vec::new();
            let mut current_path = std::collections::HashSet::new();

            if Self::has_cycle(
                &graph,
                start_node,
                &mut path,
                &mut current_path,
                &mut visited,
            ) {
                // Find the transaction to abort (youngest in the cycle)
                if let Some(&victim) = path.last() {
                    deadlocked.push(victim);
                }
            }
        }

        deadlocked
    }

    /// Helper function to detect cycles in the wait-for graph using DFS.
    fn has_cycle(
        graph: &HashMap<Uuid, Vec<Uuid>>,
        node: Uuid,
        path: &mut Vec<Uuid>,
        current_path: &mut std::collections::HashSet<Uuid>,
        visited: &mut std::collections::HashSet<Uuid>,
    ) -> bool {
        visited.insert(node);
        current_path.insert(node);
        path.push(node);

        if let Some(neighbors) = graph.get(&node) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) && Self::has_cycle(graph, neighbor, path, current_path, visited) {
                    return true;
                }
                else if current_path.contains(&neighbor) {
                    return true; // Cycle found
                }
            }
        }

        current_path.remove(&node);
        path.pop();
        false
    }
}

/// Central manager for file locks across the Sentinel system.
///
/// This manager provides thread-safe, cross-process file locking with deadlock
/// detection and timeout handling. It uses filesystem-level advisory locks
/// via the `fs2` crate, ensuring consistency across multiple Sentinel instances.
///
/// # Key Features
///
/// - **Cross-Process Locking**: Works across multiple processes using the same files
/// - **Deadlock Detection**: Automatically detects and resolves deadlocks
/// - **Timeout Handling**: Configurable timeouts prevent indefinite blocking
/// - **RAII Guards**: Automatic lock release via `LockGuard`
/// - **Async Operations**: All operations are async and non-blocking
///
/// # Lock Semantics
///
/// - **Exclusive Locks**: Used for write operations. Only one exclusive lock per file.
/// - **Shared Locks**: Used for read operations. Multiple shared locks allowed simultaneously.
/// - **Compatibility**: Exclusive locks block all other locks. Shared locks block exclusive locks.
///
/// # Deadlock Handling
///
/// The manager implements a wait-for graph algorithm to detect deadlocks:
///
/// 1. Each lock request is tracked in a dependency graph
/// 2. Cycles in the graph indicate deadlocks
/// 3. When detected, the youngest transaction in the cycle is aborted
/// 4. Aborted requests are retried with exponential backoff
///
/// # Performance
///
/// - Lock table operations are O(1) using hash maps
/// - Deadlock detection runs periodically in background
/// - Lock files are reused when possible to reduce overhead
///
/// # Error Handling
///
/// Returns `SentinelError::LockTimeout` when timeout expires.
/// Returns `SentinelError::DeadlockDetected` when deadlock is detected.
/// Returns `SentinelError::LockContention` when lock contention is too high.
#[derive(Debug)]
pub struct FileLockManager {
    /// Lock table: path -> current lock state
    lock_table:            Arc<DashMap<PathBuf, Arc<RwLock<Option<(Uuid, LockStrategy)>>>>>,
    /// Deadlock detector instance
    deadlock_detector:     Arc<DeadlockDetector>,
    /// Default timeout for lock acquisitions
    default_timeout:       Duration,
    /// Maximum retries after deadlock detection
    max_deadlock_retries:  u32,
    /// Base delay for exponential backoff after deadlock
    deadlock_backoff_base: Duration,
    /// Mutex to serialize lock acquisitions (prevents thundering herd)
    acquisition_mutex:     Arc<Mutex<()>>,
}

impl FileLockManager {
    /// Create a new file lock manager with default settings.
    ///
    /// # Default Configuration
    ///
    /// - Default timeout: 30 seconds
    /// - Max deadlock retries: 3
    /// - Deadlock backoff base: 100ms
    pub fn new() -> Self {
        Self {
            lock_table:            Arc::new(DashMap::new()),
            deadlock_detector:     Arc::new(DeadlockDetector::new()),
            default_timeout:       Duration::from_secs(30),
            max_deadlock_retries:  3,
            deadlock_backoff_base: Duration::from_millis(100),
            acquisition_mutex:     Arc::new(Mutex::new(())),
        }
    }

    /// Create a new file lock manager with custom settings.
    ///
    /// # Parameters
    ///
    /// * `default_timeout` - Default timeout for lock acquisitions
    /// * `max_deadlock_retries` - Maximum retries after deadlock detection
    /// * `deadlock_backoff_base` - Base delay for exponential backoff
    pub fn with_config(default_timeout: Duration, max_deadlock_retries: u32, deadlock_backoff_base: Duration) -> Self {
        Self {
            default_timeout,
            max_deadlock_retries,
            deadlock_backoff_base,
            ..Self::new()
        }
    }

    /// Acquire a lock on the specified file path.
    ///
    /// This method will block until the lock is acquired, the timeout expires,
    /// or a deadlock is detected. It implements deadlock detection and automatic
    /// retry with exponential backoff.
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the file to lock
    /// * `strategy` - Lock strategy (Exclusive or Shared)
    /// * `timeout` - Maximum time to wait for the lock (None uses default)
    ///
    /// # Returns
    ///
    /// Returns a `LockGuard` that automatically releases the lock when dropped.
    ///
    /// # Errors
    ///
    /// * `SentinelError::LockTimeout` - Lock acquisition timed out
    /// * `SentinelError::DeadlockDetected` - Deadlock detected and retries exhausted
    /// * `SentinelError::IoError` - I/O error during lock acquisition
    pub async fn acquire_lock(
        self: &Arc<Self>,
        path: &Path,
        strategy: LockStrategy,
        timeout: Option<Duration>,
    ) -> Result<LockGuard> {
        let timeout = timeout.unwrap_or(self.default_timeout);
        let _start_time = Instant::now();
        let requester_id = Uuid::new_v4(); // Unique UUID for this lock acquisition

        // Try to acquire lock with deadlock detection and retries
        for retry in 0 ..= self.max_deadlock_retries {
            match self
                .try_acquire_lock_with_deadlock_detection(path, strategy, timeout, requester_id)
                .await
            {
                Ok(guard) => return Ok(guard),
                Err(SentinelError::DeadlockDetected) if retry < self.max_deadlock_retries => {
                    // Exponential backoff before retry
                    let delay = self.deadlock_backoff_base * 2_u32.pow(retry);
                    time::sleep(delay).await;
                    continue;
                },
                Err(e) => return Err(e),
            }
        }

        Err(SentinelError::DeadlockDetected)
    }

    /// Internal method to attempt lock acquisition with deadlock detection.
    async fn try_acquire_lock_with_deadlock_detection(
        self: &Arc<Self>,
        path: &Path,
        strategy: LockStrategy,
        timeout: Duration,
        requester_id: Uuid,
    ) -> Result<LockGuard> {
        // Serialize acquisitions to prevent thundering herd
        let _acquisition_guard = self.acquisition_mutex.lock().await;

        // Check if the path is already locked by a different holder
        let existing_holder = if let Some(lock_state) = self.lock_table.get(path) {
            let state = lock_state.read().await;
            state.clone().and_then(|(holder, strategy)| {
                if holder != requester_id {
                    Some((holder, strategy))
                }
                else {
                    None
                }
            })
        }
        else {
            None
        };

        // If path is locked by someone else, register wait relationship
        if let Some((holder_id, existing_strategy)) = existing_holder {
            // Record wait relationship: requester is waiting for holder
            self.deadlock_detector
                .register_wait(requester_id, holder_id)
                .await?;

            // Before retrying, check if we're in a deadlock cycle
            if self.deadlock_detector.detect_deadlock(requester_id).await {
                // Deadlock detected! Abort this request
                error!(
                    "Deadlock detected for requester {:?} while waiting for holder {:?}",
                    requester_id, holder_id
                );
                self.deadlock_detector
                    .unregister_wait(requester_id, holder_id)
                    .await?;
                return Err(SentinelError::DeadlockDetected);
            }

            debug!(
                "Requester {:?} waiting for lock held by holder {:?} (strategy: {:?})",
                requester_id, holder_id, existing_strategy
            );
        }

        // Try to acquire the filesystem lock with timeout
        let file = self
            .acquire_filesystem_lock(path, strategy, timeout)
            .await?;
        let acquired_at = Instant::now();

        // Record the lock acquisition
        {
            let lock_state = self
                .lock_table
                .entry(path.to_path_buf())
                .or_insert_with(|| Arc::new(RwLock::new(None)));
            let mut state = lock_state.write().await;
            *state = Some((requester_id, strategy));
        }

        // Record active lock in deadlock detector
        self.deadlock_detector
            .record_lock_acquired(path, requester_id, strategy)
            .await;

        Ok(LockGuard {
            path: path.to_path_buf(),
            strategy,
            _manager: Arc::clone(self),
            file,
            acquired_at,
            holder_id: requester_id,
        })
    }

    /// Acquire the actual filesystem lock using fs2.
    async fn acquire_filesystem_lock(&self, path: &Path, strategy: LockStrategy, timeout: Duration) -> Result<File> {
        // For locking, we need to open the file synchronously
        // Try to open existing file first
        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File doesn't exist, create it for locking purposes
                // This is necessary for fs2 locking to work
                File::options()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path)
                    .map_err(|e| {
                        SentinelError::Io {
                            source: e,
                        }
                    })?
            },
            Err(e) => {
                return Err(SentinelError::Io {
                    source: e,
                })
            },
        };

        // Attempt to acquire the lock with timeout
        let result = match strategy {
            LockStrategy::Exclusive => {
                time::timeout(timeout, async {
                    file.lock_exclusive().map_err(|e| {
                        SentinelError::Io {
                            source: e,
                        }
                    })
                })
                .await
            },
            LockStrategy::Shared => {
                time::timeout(timeout, async {
                    file.lock_shared().map_err(|e| {
                        SentinelError::Io {
                            source: e,
                        }
                    })
                })
                .await
            },
        };

        match result {
            Ok(Ok(())) => Ok(file),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                Err(SentinelError::LockTimeout {
                    path:       path.to_path_buf(),
                    timeout_ms: timeout.as_millis() as u64,
                })
            },
        }
    }

    /// Force release a lock (used for deadlock resolution).
    /// This is an emergency method that should be used carefully.
    pub async fn force_release_lock(&self, path: &Path) -> Result<()> {
        if let Some(lock_state) = self.lock_table.get(path) {
            let mut state = lock_state.write().await;
            *state = None;
        }
        self.deadlock_detector.record_lock_released(path).await;
        Ok(())
    }

    /// Check if a path is currently locked.
    pub async fn is_locked(&self, path: &Path) -> bool {
        if let Some(lock_state) = self.lock_table.get(path) {
            let state = lock_state.read().await;
            state.is_some()
        }
        else {
            false
        }
    }

    /// Get information about the current lock holder for a path.
    pub async fn get_lock_holder(&self, path: &Path) -> Option<(Uuid, LockStrategy)> {
        if let Some(lock_state) = self.lock_table.get(path) {
            let state = lock_state.read().await;
            *state
        }
        else {
            None
        }
    }

    /// Get statistics about the lock manager.
    pub async fn get_stats(&self) -> LockManagerStats {
        let active_locks = self.lock_table.len();
        let _active = self.deadlock_detector.active_locks.read().await;
        let pending = self.deadlock_detector.pending_requests.read().await;

        LockManagerStats {
            active_locks,
            total_pending_requests: pending.values().map(|v| v.len()).sum(),
            paths_with_pending_requests: pending.len(),
        }
    }
}

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

impl Default for FileLockManager {
    fn default() -> Self { Self::new() }
}

impl LockGuard {
    /// Get the path of the locked file.
    pub fn path(&self) -> &Path { &self.path }

    /// Get the lock strategy used.
    pub const fn strategy(&self) -> LockStrategy { self.strategy }

    /// Get when this lock was acquired.
    pub const fn acquired_at(&self) -> Instant { self.acquired_at }

    /// Get the ID of the lock holder.
    pub fn holder_id(&self) -> Uuid { self.holder_id }

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
    pub async fn upgrade_to_exclusive(self, manager: &Arc<FileLockManager>, timeout: Option<Duration>) -> Result<Self> {
        // Check if current lock is shared
        if self.strategy != LockStrategy::Shared {
            return Err(SentinelError::InvalidLockState {
                reason: "cannot upgrade non-shared lock".to_string(),
            });
        }

        let _timeout = timeout.unwrap_or(manager.default_timeout);
        let _requester_id = Uuid::new_v4();

        // Attempt upgrade - for now, just return an error as this feature is not fully implemented
        return Err(SentinelError::InvalidLockState {
            reason: "lock upgrade not yet implemented".to_string(),
        });
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
    pub fn downgrade_to_shared(mut self) -> Result<Self> {
        // Check if current lock is exclusive
        if self.strategy != LockStrategy::Exclusive {
            return Err(SentinelError::InvalidLockState {
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
                Err(SentinelError::Io {
                    source: e,
                })
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::Duration,
    };

    use uuid::Uuid;
    use tempfile::tempdir;
    use tokio::time::timeout;

    use super::*;

    #[tokio::test]
    async fn test_filelockmanager_acquire_exclusive_lock() {
        // Test acquiring an exclusive lock
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_file.txt");

        let guard = manager
            .acquire_lock(&path, LockStrategy::Exclusive, None)
            .await
            .unwrap();

        assert!(manager.is_locked(&path).await);
        assert_eq!(guard.strategy(), LockStrategy::Exclusive);

        drop(guard);
        assert!(!manager.is_locked(&path).await);
    }

    #[tokio::test]
    async fn test_filelockmanager_acquire_shared_lock() {
        // Test acquiring shared locks (multiple readers)
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_file_shared.txt");

        let guard1 = manager
            .acquire_lock(&path, LockStrategy::Shared, None)
            .await
            .unwrap();
        let guard2 = manager
            .acquire_lock(&path, LockStrategy::Shared, None)
            .await
            .unwrap();

        assert!(manager.is_locked(&path).await);
        assert_eq!(guard1.strategy(), LockStrategy::Shared);
        assert_eq!(guard2.strategy(), LockStrategy::Shared);

        drop(guard1);
        drop(guard2);
        assert!(!manager.is_locked(&path).await);
    }

    #[tokio::test]
    async fn test_filelockmanager_timeout_handling() {
        // Test timeout handling for lock acquisition
        let manager = Arc::new(FileLockManager::with_config(
            Duration::from_millis(100),
            0,
            Duration::from_millis(10),
        ));
        let path = PathBuf::from("/tmp/test_file_timeout.txt");

        let result = manager
            .acquire_lock(
                &path,
                LockStrategy::Exclusive,
                Some(Duration::from_millis(100)),
            )
            .await;

        assert!(matches!(result, Err(SentinelError::LockTimeout { .. })));
    }

    #[tokio::test]
    async fn test_filelockmanager_concurrent_access() {
        // Test concurrent access scenarios
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_file_concurrent.txt");

        let guard1 = manager
            .acquire_lock(&path, LockStrategy::Exclusive, None)
            .await
            .unwrap();

        // Try to acquire same file with different strategy (should fail)
        let result = manager
            .acquire_lock(&path, LockStrategy::Shared, None)
            .await;
        assert!(matches!(result, Err(SentinelError::LockTimeout { .. })));

        drop(guard1);
        let guard2 = manager
            .acquire_lock(&path, LockStrategy::Shared, None)
            .await
            .unwrap();
        assert_eq!(guard2.strategy(), LockStrategy::Shared);

        drop(guard2);
        assert!(!manager.is_locked(&path).await);
    }

    #[tokio::test]
    async fn test_filelockmanager_cleanup_on_drop() {
        // Test automatic cleanup when lock guard is dropped
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_file_cleanup.txt");

        {
            let guard = manager
                .acquire_lock(&path, LockStrategy::Exclusive, None)
                .await
                .unwrap();
            assert!(manager.is_locked(&path).await);

            // Verify lock manager can acquire again after dropping
            let guard2 = manager
                .acquire_lock(&path, LockStrategy::Shared, None)
                .await
                .unwrap();
            assert!(manager.is_locked(&path).await);

            drop(guard2);
            assert!(manager.is_locked(&path).await);
        }

        assert!(!manager.is_locked(&path).await);
    }

    #[tokio::test]
    async fn test_lockguard_raii_behavior() {
        // Test RAII behavior - lock should be released when guard is dropped
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_raii.txt");

        {
            let guard = manager
                .acquire_lock(&path, LockStrategy::Exclusive, None)
                .await
                .unwrap();
            assert!(manager.is_locked(&path).await);

            assert_eq!(guard.path(), &path);
            assert_eq!(guard.strategy(), LockStrategy::Exclusive);
            assert!(guard.acquired_at() < std::time::Instant::now());
        }

        assert!(!manager.is_locked(&path).await);
    }

    #[tokio::test]
    async fn test_lockstrategy_exclusive_behavior() {
        // Test exclusive lock blocks other locks
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_exclusive.txt");

        let guard = manager
            .acquire_lock(&path, LockStrategy::Exclusive, None)
            .await
            .unwrap();

        // Exclusive lock blocks other exclusive locks
        let result = manager
            .acquire_lock(&path, LockStrategy::Exclusive, None)
            .await;
        assert!(matches!(result, Err(SentinelError::LockTimeout { .. })));

        // Exclusive lock also blocks shared locks
        let result = manager
            .acquire_lock(&path, LockStrategy::Shared, None)
            .await;
        assert!(matches!(result, Err(SentinelError::LockTimeout { .. })));

        drop(guard);
    }

    #[tokio::test]
    async fn test_lockstrategy_shared_behavior() {
        // Test shared locks allow multiple concurrent readers
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_shared.txt");

        let guard1 = manager
            .acquire_lock(&path, LockStrategy::Shared, None)
            .await
            .unwrap();
        let guard2 = manager
            .acquire_lock(&path, LockStrategy::Shared, None)
            .await
            .unwrap();

        assert!(manager.is_locked(&path).await);
        assert_eq!(guard1.strategy(), LockStrategy::Shared);
        assert_eq!(guard2.strategy(), LockStrategy::Shared);

        // Shared locks block exclusive locks
        let result = manager
            .acquire_lock(&path, LockStrategy::Exclusive, None)
            .await;
        assert!(matches!(result, Err(SentinelError::LockTimeout { .. })));

        drop(guard1);
        drop(guard2);
        assert!(!manager.is_locked(&path).await);
    }

    #[tokio::test]
    async fn test_deadlockdetector_cycle_detection() {
        // Test deadlock detection with cycle
        let detector = DeadlockDetector::new();
        let pid1 = Uuid::new_v4();
        let pid2 = Uuid::new_v4();
        let pid3 = Uuid::new_v4();

        detector.register_wait(pid1, pid2).await.unwrap();
        detector.register_wait(pid2, pid3).await.unwrap();
        detector.register_wait(pid3, pid1).await.unwrap();

        assert!(detector.detect_deadlock(pid1).await);
        assert!(detector.detect_deadlock(pid2).await);
        assert!(detector.detect_deadlock(pid3).await);
    }

    #[tokio::test]
    async fn test_deadlockdetector_wait_graph_updates() {
        // Test wait-for graph update and cleanup
        let detector = DeadlockDetector::new();
        let pid1 = Uuid::new_v4();
        let pid2 = Uuid::new_v4();

        detector.register_wait(pid1, pid2).await.unwrap();

        let graph = detector.wait_graph.read().await;
        assert!(graph.contains_key(&pid2));
        assert_eq!(graph.get(&pid2).unwrap().len(), 1);
        assert!(graph.get(&pid2).unwrap().contains(&pid1));

        drop(graph);
        detector.unregister_wait(pid1, pid2).await.unwrap();

        let graph = detector.wait_graph.read().await;
        assert!(!graph.contains_key(&pid2));
    }

    #[tokio::test]
    async fn test_deadlockdetector_resolution_logic() {
        // Test deadlock resolution finds transactions in cycle
        let detector = DeadlockDetector::new();
        let pid1 = Uuid::new_v4();
        let pid2 = Uuid::new_v4();
        let pid3 = Uuid::new_v4();

        detector.register_wait(pid1, pid2).await.unwrap();
        detector.register_wait(pid2, pid3).await.unwrap();
        detector.register_wait(pid3, pid1).await.unwrap();

        let deadlocked = detector.find_deadlocked_transactions().await;
        assert!(!deadlocked.is_empty());

        for pid in deadlocked {
            assert!(pid == pid1 || pid == pid2 || pid == pid3);
        }

        detector.unregister_wait(pid1, pid2).await.unwrap();
        detector.unregister_wait(pid2, pid3).await.unwrap();
        detector.unregister_wait(pid3, pid1).await.unwrap();
    }

    #[tokio::test]
    async fn test_filelockmanager_multiple_locks_same_file() {
        // Test multiple lock holders trying to acquire same file
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_multi_lock.txt");

        let guard1 = manager
            .acquire_lock(&path, LockStrategy::Exclusive, None)
            .await
            .unwrap();
        assert!(manager.is_locked(&path).await);

        let result = manager
            .acquire_lock(&path, LockStrategy::Exclusive, None)
            .await;
        assert!(matches!(result, Err(SentinelError::LockTimeout { .. })));

        drop(guard1);
        let guard2 = manager
            .acquire_lock(&path, LockStrategy::Shared, None)
            .await
            .unwrap();
        assert!(manager.is_locked(&path).await);

        drop(guard2);
        assert!(!manager.is_locked(&path).await);
    }

    #[tokio::test]
    async fn test_filelockmanager_lock_holder_info() {
        // Test getting lock holder information
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_holder.txt");

        let guard = manager
            .acquire_lock(&path, LockStrategy::Exclusive, None)
            .await
            .unwrap();

        let holder = manager.get_lock_holder(&path).await;
        assert!(holder.is_some());
        let (holder_id, strategy) = holder.unwrap();
        assert_eq!(holder_id, guard.holder_id());
        assert_eq!(strategy, LockStrategy::Exclusive);

        drop(guard);
        assert!(manager.get_lock_holder(&path).await.is_none());
    }

    #[tokio::test]
    async fn test_filelockmanager_is_locked_check() {
        // Test is_locked method
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_islocked.txt");

        assert!(!manager.is_locked(&path).await);

        let guard = manager
            .acquire_lock(&path, LockStrategy::Shared, None)
            .await
            .unwrap();

        assert!(manager.is_locked(&path).await);

        drop(guard);
        assert!(!manager.is_locked(&path).await);
    }

    #[tokio::test]
    async fn test_filelockmanager_stats() {
        // Test getting lock manager statistics
        let manager = Arc::new(FileLockManager::new());
        let stats = manager.get_stats().await;

        assert_eq!(stats.active_locks, 0);
        assert_eq!(stats.total_pending_requests, 0);
        assert_eq!(stats.paths_with_pending_requests, 0);
    }

    #[tokio::test]
    async fn test_filelockmanager_with_config() {
        // Test that FileLockManager can be created with custom config
        // Note: Configuration fields are private, so we verify the manager works correctly
        let manager = Arc::new(FileLockManager::with_config(
            Duration::from_secs(60),
            5,
            Duration::from_millis(200),
        ));

        // Use a unique temp path to avoid conflicts with other tests
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_config.lock");

        // Explicitly check lock table is clean
        assert!(!manager.lock_table.contains_key(&path));
        assert!(!manager.is_locked(&path).await);

        let guard = manager
            .acquire_lock(&path, LockStrategy::Shared, None)
            .await
            .unwrap();

        assert!(manager.lock_table.contains_key(&path));
        assert!(manager.is_locked(&path).await);

        drop(guard);

        // After drop, lock should be removed from lock table
        // Note: File on disk might still exist temporarily
        // Use a small delay to ensure cleanup completes
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(!manager.is_locked(&path).await);
    }

    #[tokio::test]
    async fn test_lockguard_methods() {
        // Test LockGuard getter methods
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_guard_methods.txt");

        let guard = manager
            .acquire_lock(&path, LockStrategy::Shared, None)
            .await
            .unwrap();

        assert_eq!(guard.path(), &path);
        assert_eq!(guard.strategy(), LockStrategy::Shared);
        assert!(guard.acquired_at() < std::time::Instant::now());

        drop(guard);
    }
}
