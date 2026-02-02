//! # File Lock Manager
//!
//! Central manager for file locks across the Sentinel system.
//! Provides thread-safe, cross-process file locking with deadlock detection.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{atomic::AtomicU64, Arc},
    time::{Duration, Instant},
};

use dashmap::DashMap;
use fs2::FileExt;
use futures::channel::oneshot;
use tokio::{sync::Mutex, time};
use tracing::{debug, error};

use crate::error::{Result, SentinelError};
use super::{deadlock_detector::DeadlockDetector, lock_guard::LockGuard, lock_strategy::LockStrategy};

/// State of a lock on a specific path
#[derive(Debug, Clone)]
pub enum LockState {
    /// No lock held
    None,
    /// Exclusive lock held by single holder
    Exclusive(String),
    /// Shared locks held by multiple holders
    Shared(HashSet<String>),
}

/// Entry in the lock queue representing a waiting requester.
#[derive(Debug)]
pub struct LockQueueEntry {
    /// Unique ID of the requester
    pub requester_id: String,
    /// Lock strategy requested
    pub strategy:     LockStrategy,
    /// Timeout for this request
    pub timeout:      Duration,
    /// Channel to wake up the requester when the lock becomes available
    pub waker:        oneshot::Sender<()>,
}

/// FIFO queue for managing waiting lock requesters.
///
/// This queue ensures fair scheduling by waking up waiters in the order
/// they requested the lock. It prevents starvation and provides predictable
/// lock acquisition behavior under contention.
#[derive(Debug)]
pub struct LockQueue {
    /// FIFO queue of waiting requesters
    queue: VecDeque<LockQueueEntry>,
}

impl LockQueue {
    /// Create a new empty lock queue.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Add a requester to the end of the queue.
    pub fn enqueue(&mut self, entry: LockQueueEntry) { self.queue.push_back(entry); }

    /// Remove and return the first requester from the queue.
    pub fn dequeue(&mut self) -> Option<LockQueueEntry> { self.queue.pop_front() }

    /// Return a reference to the first requester without removing it.
    pub fn peek(&self) -> Option<&LockQueueEntry> { self.queue.front() }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool { self.queue.is_empty() }

    /// Get the number of waiting requesters.
    pub fn len(&self) -> usize { self.queue.len() }

    /// Clear all entries from the queue.
    pub fn clear(&mut self) { self.queue.clear(); }
}

impl Default for LockQueue {
    fn default() -> Self { Self::new() }
}

/// Central manager for file locks across the Sentinel system.
///
/// This manager provides thread-safe, cross-process file locking with deadlock
/// detection and timeout handling. It uses filesystem-level locking via `fs2` crate
/// which provides cross-platform advisory file locking.
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
/// Returns `SentinelError::IoError` - I/O error during lock acquisition.
#[derive(Debug)]
#[allow(
    clippy::field_scoped_visibility_modifiers,
    reason = "pub(crate) fields are used internally within the crate and are appropriate for internal visibility"
)]
pub struct FileLockManager {
    /// Lock table: path -> current lock state (tracks all holders)
    pub(crate) lock_table:            Arc<DashMap<PathBuf, Arc<std::sync::Mutex<LockState>>>>,
    /// Lock queues: path -> FIFO queue of waiting requesters
    pub(crate) lock_queues:           Arc<DashMap<PathBuf, Arc<std::sync::Mutex<LockQueue>>>>,
    /// Deadlock detector instance
    pub(crate) deadlock_detector:     Arc<DeadlockDetector>,
    /// Default timeout for lock acquisitions
    pub(crate) default_timeout:       Duration,
    /// Maximum retries after deadlock detection
    pub(crate) max_deadlock_retries:  u32,
    /// Base delay for exponential backoff after deadlock
    pub(crate) deadlock_backoff_base: Duration,
    /// Mutex to serialize lock acquisitions (prevents thundering herd)
    pub(crate) acquisition_mutex:     Arc<Mutex<()>>,
    /// Performance metrics counters
    pub(crate) lock_acquisitions:     Arc<AtomicU64>,
    pub(crate) lock_contentions:      Arc<AtomicU64>,
    pub(crate) failed_acquisitions:   Arc<AtomicU64>,
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
            lock_queues:           Arc::new(DashMap::new()),
            deadlock_detector:     Arc::new(DeadlockDetector::new()),
            default_timeout:       Duration::from_secs(30),
            max_deadlock_retries:  3,
            deadlock_backoff_base: Duration::from_millis(100),
            acquisition_mutex:     Arc::new(Mutex::new(())),
            lock_acquisitions:     Arc::new(AtomicU64::new(0)),
            lock_contentions:      Arc::new(AtomicU64::new(0)),
            failed_acquisitions:   Arc::new(AtomicU64::new(0)),
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
        let requester_id = cuid2::cuid(); // Unique UUID for this lock acquisition

        // Try to acquire lock with deadlock detection and retries
        for retry in 0 ..= self.max_deadlock_retries {
            match self
                .try_acquire_lock_with_deadlock_detection(path, strategy, timeout, requester_id.clone())
                .await
            {
                Ok(guard) => return Ok(guard),
                Err(SentinelError::DeadlockDetected) if retry < self.max_deadlock_retries => {
                    // Exponential backoff before retry
                    let delay = self.deadlock_backoff_base * 2u32.pow(retry);
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
        requester_id: String,
    ) -> Result<LockGuard> {
        // Serialize lock table checks to prevent thundering herd
        // But release before filesystem lock acquisition
        let (is_locked, is_compatible, holders) = {
            let _acquisition_guard = self.acquisition_mutex.lock().await;

            // Check if the path is already locked and determine compatibility
            if let Some(lock_state_ref) = self.lock_table.get(path) {
                let state = lock_state_ref.lock().unwrap();
                match &*state {
                    LockState::None => (false, true, Vec::new()),
                    LockState::Exclusive(holder) if holder != &requester_id => {
                        // Exclusive lock held by someone else - not compatible
                        (true, false, vec![holder.clone()])
                    },
                    LockState::Exclusive(_) => {
                        // We already hold the exclusive lock (reentrant)
                        (true, true, Vec::new())
                    },
                    LockState::Shared(holders) if !holders.contains(&requester_id) => {
                        // Shared locks held - compatible only if we're requesting shared
                        match strategy {
                            LockStrategy::Shared => (true, true, Vec::new()),
                            LockStrategy::Exclusive => (true, false, holders.iter().cloned().collect()),
                        }
                    },
                    LockState::Shared(_) => {
                        // We already hold a shared lock (reentrant)
                        (true, true, Vec::new())
                    },
                }
            }
            else {
                (false, true, Vec::new())
            }
            // _acquisition_guard is dropped here, releasing the mutex
        };

        // If path is locked by someone else and not compatible, enqueue this requester
        if is_locked && !is_compatible {
            // Record wait relationships for all current holders
            for holder_id in &holders {
                self.deadlock_detector
                    .register_wait(requester_id.clone(), holder_id.clone())
                    .await?;
            }

            // Before enqueuing, check if we're in a deadlock cycle
            if self
                .deadlock_detector
                .detect_deadlock(requester_id.clone())
                .await
            {
                // Deadlock detected! Abort this request
                error!(
                    "Deadlock detected for requester {:?} while waiting for holders {:?}",
                    requester_id, holders
                );
                for holder_id in &holders {
                    self.deadlock_detector
                        .unregister_wait(requester_id.clone(), holder_id.clone())
                        .await?;
                }
                return Err(SentinelError::DeadlockDetected);
            }

            debug!(
                "Requester {:?} enqueuing for lock held by holders {:?}",
                requester_id, holders
            );

            // Enqueue this requester and wait
            return self
                .enqueue_and_wait(path, strategy, timeout, requester_id, holders)
                .await;
        }

        // Path is not locked or locks are compatible, try to acquire immediately
        // Note: acquisition_mutex is released at this point, allowing other threads to check locks
        match self.acquire_filesystem_lock(path, strategy, timeout).await {
            Ok(file) => {
                let acquired_at = Instant::now();

                // Record the lock acquisition
                {
                    let lock_state = self
                        .lock_table
                        .entry(path.to_path_buf())
                        .or_insert_with(|| Arc::new(std::sync::Mutex::new(LockState::None)));
                    let mut state = lock_state.lock().unwrap();

                    *state = match strategy {
                        LockStrategy::Exclusive => LockState::Exclusive(requester_id.clone()),
                        LockStrategy::Shared => {
                            match &*state {
                                LockState::Shared(existing) => {
                                    let mut holders = existing.clone();
                                    holders.insert(requester_id.clone());
                                    LockState::Shared(holders)
                                },
                                _ => {
                                    let mut holders = HashSet::new();
                                    holders.insert(requester_id.clone());
                                    LockState::Shared(holders)
                                },
                            }
                        },
                    };
                }

                // Record active lock in deadlock detector
                self.deadlock_detector
                    .record_lock_acquired(path, requester_id.clone(), strategy)
                    .await;

                // Increment acquisition counter
                self.lock_acquisitions
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                Ok(LockGuard {
                    path: path.to_path_buf(),
                    strategy,
                    _manager: Arc::clone(self),
                    file,
                    acquired_at,
                    holder_id: requester_id,
                })
            },
            Err(SentinelError::LockTimeout {
                ..
            }) => {
                // Could not acquire immediately, enqueue and wait
                // Increment failed acquisitions counter for immediate timeout
                self.failed_acquisitions
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.enqueue_and_wait(path, strategy, timeout, requester_id, Vec::new())
                    .await
            },
            Err(e) => Err(e),
        }
    }

    /// Enqueue a requester and wait for the lock to become available.
    async fn enqueue_and_wait(
        self: &Arc<Self>,
        path: &Path,
        strategy: LockStrategy,
        timeout: Duration,
        requester_id: String,
        holders: Vec<String>,
    ) -> Result<LockGuard> {
        // Create a channel to wait for wakeup
        let (sender, receiver) = oneshot::channel();

        // Create queue entry
        let entry = LockQueueEntry {
            requester_id: requester_id.clone(),
            strategy,
            timeout,
            waker: sender,
        };

        // Add to queue for this path
        {
            let queue = self
                .lock_queues
                .entry(path.to_path_buf())
                .or_insert_with(|| Arc::new(std::sync::Mutex::new(LockQueue::new())));
            let mut queue = queue.lock().unwrap();
            queue.enqueue(entry);
        }

        // Increment contention counter
        self.lock_contentions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        debug!("Enqueued requester {:?} for path {:?}", requester_id, path);

        // Wait for wakeup with timeout
        match time::timeout(timeout, receiver).await {
            Ok(Ok(())) => {
                // Woken up! Remove ourselves from queue and try to acquire the lock
                debug!(
                    "Requester {:?} woken up, removing from queue and attempting lock acquisition",
                    requester_id
                );
                self.remove_from_queue(path, &requester_id).await;
                self.try_acquire_lock_immediately(path, strategy, requester_id)
                    .await
            },
            Ok(Err(_)) => {
                // Channel closed (shouldn't happen in normal operation)
                debug!("Channel closed for requester {:?}", requester_id);
                Err(SentinelError::LockTimeout {
                    path:       path.to_path_buf(),
                    timeout_ms: timeout.as_millis() as u64,
                })
            },
            Err(_) => {
                // Timeout - remove from queue and return error
                debug!("Timeout waiting in queue for requester {:?}", requester_id);
                self.remove_from_queue(path, &requester_id).await;

                // Unregister wait relationships from deadlock detector
                for holder_id in &holders {
                    let _ = self
                        .deadlock_detector
                        .unregister_wait(requester_id.clone(), holder_id.clone())
                        .await;
                }

                // Increment failed acquisitions counter
                self.failed_acquisitions
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                Err(SentinelError::LockTimeout {
                    path:       path.to_path_buf(),
                    timeout_ms: timeout.as_millis() as u64,
                })
            },
        }
    }

    /// Try to acquire lock immediately (used after being woken from queue).
    async fn try_acquire_lock_immediately(
        self: &Arc<Self>,
        path: &Path,
        strategy: LockStrategy,
        requester_id: String,
    ) -> Result<LockGuard> {
        // Try to acquire the filesystem lock (should succeed since we were woken)
        // Use a longer timeout here since the previous holder just released the lock
        // but the OS may need time to fully release it
        let file = self
            .acquire_filesystem_lock(path, strategy, Duration::from_secs(5))
            .await?;
        let acquired_at = Instant::now();

        // Record the lock acquisition
        {
            let lock_state = self
                .lock_table
                .entry(path.to_path_buf())
                .or_insert_with(|| Arc::new(std::sync::Mutex::new(LockState::None)));
            let mut state = lock_state.lock().unwrap();

            *state = match strategy {
                LockStrategy::Exclusive => LockState::Exclusive(requester_id.clone()),
                LockStrategy::Shared => {
                    match &*state {
                        LockState::Shared(existing) => {
                            let mut holders = existing.clone();
                            holders.insert(requester_id.clone());
                            LockState::Shared(holders)
                        },
                        _ => {
                            let mut holders = HashSet::new();
                            holders.insert(requester_id.clone());
                            LockState::Shared(holders)
                        },
                    }
                },
            };
        }

        // Record active lock in deadlock detector
        self.deadlock_detector
            .record_lock_acquired(path, requester_id.clone(), strategy)
            .await;

        // Increment acquisition counter
        self.lock_acquisitions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(LockGuard {
            path: path.to_path_buf(),
            strategy,
            _manager: Arc::clone(self),
            file,
            acquired_at,
            holder_id: requester_id,
        })
    }

    /// Remove a specific requester from the queue for a path.
    async fn remove_from_queue(&self, path: &Path, requester_id: &str) {
        if let Some(queue) = self.lock_queues.get(path) {
            let mut queue = queue.lock().unwrap();
            // Remove the specific requester (inefficient but correct)
            queue
                .queue
                .retain(|entry| entry.requester_id != requester_id);
        }
    }

    /// Wake up the next waiter in the queue for a given path.
    /// This should be called when a lock is released.
    pub(crate) fn wake_next_waiter(&self, path: &Path) {
        if let Some(queue_ref) = self.lock_queues.get(path) {
            if let Ok(mut queue) = queue_ref.try_lock() {
                if let Some(entry) = queue.dequeue() {
                    debug!(
                        "Waking up next waiter {:?} for path {:?}",
                        entry.requester_id, path
                    );
                    // Send wakeup signal (ignore if receiver is dropped)
                    let _ = entry.waker.send(());
                }
            }
        }
    }

    /// Acquire the actual filesystem lock using fs2.
    async fn acquire_filesystem_lock(
        &self,
        path: &Path,
        strategy: LockStrategy,
        timeout: Duration,
    ) -> Result<fs::File> {
        let path_owned = path.to_path_buf();

        // Open the file in a blocking task to avoid blocking the tokio runtime
        let file = tokio::task::spawn_blocking(move || {
            // Try to open existing file first
            match fs::File::open(&path_owned) {
                Ok(file) => Ok(file),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // File doesn't exist, create it for locking purposes
                    // This is necessary for fs2 locking to work
                    fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .open(&path_owned)
                        .map_err(|e| {
                            SentinelError::Io {
                                source: e,
                            }
                        })
                },
                Err(e) => {
                    Err(SentinelError::Io {
                        source: e,
                    })
                },
            }
        })
        .await
        .map_err(|e| {
            SentinelError::Io {
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            }
        })??;

        // Use try_lock with polling and timeout instead of blocking lock
        // This allows us to respect the timeout and not block indefinitely
        let deadline = Instant::now() + timeout;
        let poll_interval = Duration::from_millis(10);

        loop {
            // Try to acquire the lock (non-blocking)
            let result = match strategy {
                LockStrategy::Exclusive => {
                    file.try_lock_exclusive()
                        .map_err(|e| std::io::Error::from(e))
                },
                LockStrategy::Shared => file.try_lock_shared().map_err(|e| std::io::Error::from(e)),
            };

            match result {
                Ok(()) => {
                    // Lock acquired successfully
                    return Ok(file);
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Lock is held by someone else, check if we've timed out
                    if Instant::now() >= deadline {
                        return Err(SentinelError::LockTimeout {
                            path:       path.to_path_buf(),
                            timeout_ms: timeout.as_millis() as u64,
                        });
                    }

                    // Wait a bit before trying again
                    time::sleep(poll_interval).await;
                },
                Err(e) => {
                    // Some other error occurred
                    return Err(SentinelError::Io {
                        source: e,
                    });
                },
            }
        }
    }

    /// Force release a lock (used for deadlock resolution).
    /// This is an emergency method that should be used carefully.
    pub async fn force_release_lock(&self, path: &Path) -> Result<()> {
        if let Some(lock_state) = self.lock_table.get(path) {
            let mut state = lock_state.lock().unwrap();
            *state = LockState::None;
        }
        self.deadlock_detector.record_lock_released(path).await;
        Ok(())
    }

    /// Check if a path is currently locked.
    pub async fn is_locked(&self, path: &Path) -> bool {
        if let Some(lock_state) = self.lock_table.get(path) {
            let state = lock_state.lock().unwrap();
            !matches!(*state, LockState::None)
        }
        else {
            false
        }
    }

    /// Get information about the current lock holder for a path.
    pub async fn get_lock_holder(&self, path: &Path) -> Option<(String, LockStrategy)> {
        if let Some(lock_state) = self.lock_table.get(path) {
            let state = lock_state.lock().unwrap();
            match &*state {
                LockState::None => None,
                LockState::Exclusive(holder) => Some((holder.clone(), LockStrategy::Exclusive)),
                LockState::Shared(holders) => {
                    // Return first holder for backward compatibility
                    holders
                        .iter()
                        .next()
                        .map(|h| (h.clone(), LockStrategy::Shared))
                },
            }
        }
        else {
            None
        }
    }

    /// Get statistics about the lock manager.
    pub async fn get_stats(&self) -> super::lock_stats::LockManagerStats {
        // Count only entries that have active locks (not None state)
        let active_locks = self
            .lock_table
            .iter()
            .filter(|entry| {
                let state = entry.value().lock().unwrap();
                !matches!(*state, LockState::None)
            })
            .count();

        // Count actual queued requests and build queue lengths map
        let mut total_pending_requests = 0;
        let mut paths_with_pending_requests = 0;
        let mut queue_lengths = HashMap::new();
        for queue_ref in self.lock_queues.iter() {
            let queue_len = queue_ref.value().lock().unwrap().len();
            if queue_len > 0 {
                total_pending_requests += queue_len;
                paths_with_pending_requests += 1;
                queue_lengths.insert(queue_ref.key().to_string_lossy().to_string(), queue_len);
            }
        }

        super::lock_stats::LockManagerStats {
            active_locks,
            total_pending_requests,
            paths_with_pending_requests,
            queue_lengths,
            total_wait_time: tokio::time::Duration::ZERO, // TODO: Implement wait time tracking
            lock_acquisitions: self
                .lock_acquisitions
                .load(std::sync::atomic::Ordering::Relaxed),
            lock_contentions: self
                .lock_contentions
                .load(std::sync::atomic::Ordering::Relaxed),
            failed_acquisitions: self
                .failed_acquisitions
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

impl Default for FileLockManager {
    fn default() -> Self { Self::new() }
}
