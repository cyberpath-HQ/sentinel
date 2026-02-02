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
//! - **Exclusive Locks**: Single writer, blocks readers and writers (for write operations)
//! - **Shared Locks**: Multiple concurrent readers, blocks writers (for read operations)
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

// Public re-exports
pub use lock_strategy::LockStrategy;
pub use lock_guard::LockGuard;
pub use deadlock_detector::{DeadlockDetector, LockRequest};
pub use file_lock_manager::FileLockManager;
pub use lock_stats::LockManagerStats;

// Module declarations
mod deadlock_detector;
mod file_lock_manager;
mod lock_guard;
mod lock_stats;
mod lock_strategy;

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::Arc,
        time::Duration,
    };

    use serial_test::serial;
    use tempfile::tempdir;
    use tokio::time::timeout;

    use super::*;

    #[tokio::test]
    #[serial]
    #[serial]
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
    #[serial]
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
    #[serial]
    async fn test_filelockmanager_timeout_handling() {
        // Test timeout handling when lock is contended
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_timeout.txt");

        // First, hold an exclusive lock
        let guard = manager
            .acquire_lock(&path, LockStrategy::Exclusive, None)
            .await
            .unwrap();

        // Now try to acquire the same lock with a short timeout - should fail
        let result = manager
            .acquire_lock(
                &path,
                LockStrategy::Exclusive,
                Some(Duration::from_millis(50)),
            )
            .await;

        assert!(matches!(
            result,
            Err(crate::error::SentinelError::LockTimeout { .. })
        ));

        drop(guard);
    }

    #[tokio::test]
    #[serial]
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
            .acquire_lock(
                &path,
                LockStrategy::Shared,
                Some(Duration::from_millis(100)),
            )
            .await;
        assert!(matches!(
            result,
            Err(crate::error::SentinelError::LockTimeout { .. })
        ));

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
    #[serial]
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

            // Try to acquire shared lock while exclusive is held (should fail)
            let result = manager
                .acquire_lock(
                    &path,
                    LockStrategy::Shared,
                    Some(Duration::from_millis(100)),
                )
                .await;
            assert!(matches!(
                result,
                Err(crate::error::SentinelError::LockTimeout { .. })
            ));

            // Drop the exclusive lock
        }

        // Now shared lock should work after exclusive lock is released
        let guard2 = manager
            .acquire_lock(&path, LockStrategy::Shared, None)
            .await
            .unwrap();
        assert!(manager.is_locked(&path).await);

        drop(guard2);
        assert!(!manager.is_locked(&path).await);
    }

    #[tokio::test]
    #[serial]
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
    #[serial]
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
            .acquire_lock(
                &path,
                LockStrategy::Exclusive,
                Some(Duration::from_millis(100)),
            )
            .await;
        assert!(matches!(
            result,
            Err(crate::error::SentinelError::LockTimeout { .. })
        ));

        // Exclusive lock also blocks shared locks
        let result = manager
            .acquire_lock(
                &path,
                LockStrategy::Shared,
                Some(Duration::from_millis(100)),
            )
            .await;
        assert!(matches!(
            result,
            Err(crate::error::SentinelError::LockTimeout { .. })
        ));

        drop(guard);
    }

    #[tokio::test]
    #[serial]
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
            .acquire_lock(
                &path,
                LockStrategy::Exclusive,
                Some(Duration::from_millis(100)),
            )
            .await;
        assert!(matches!(
            result,
            Err(crate::error::SentinelError::LockTimeout { .. })
        ));

        drop(guard1);
        drop(guard2);
        assert!(!manager.is_locked(&path).await);
    }

    #[tokio::test]
    #[serial]
    async fn test_deadlockdetector_cycle_detection() {
        // Test deadlock detection with cycle
        let detector = DeadlockDetector::new();
        let pid1 = cuid2::cuid();
        let pid2 = cuid2::cuid();
        let pid3 = cuid2::cuid();

        detector
            .register_wait(pid1.clone(), pid2.clone())
            .await
            .unwrap();
        detector
            .register_wait(pid2.clone(), pid3.clone())
            .await
            .unwrap();
        detector
            .register_wait(pid3.clone(), pid1.clone())
            .await
            .unwrap();

        assert!(detector.detect_deadlock(pid1.clone()).await);
        assert!(detector.detect_deadlock(pid2.clone()).await);
        assert!(detector.detect_deadlock(pid3).await);
    }

    #[tokio::test]
    #[serial]
    async fn test_deadlockdetector_wait_graph_operations() {
        // Test wait graph registration and unregistration using public API
        let detector = DeadlockDetector::new();
        let pid1 = cuid2::cuid();
        let pid2 = cuid2::cuid();

        // Register a wait
        detector
            .register_wait(pid1.clone(), pid2.clone())
            .await
            .unwrap();

        // Verify that the wait is registered by checking for potential deadlocks
        // (two processes waiting creates a potential deadlock scenario)
        let deadlocked = detector.find_deadlocked_transactions().await;
        assert!(deadlocked.is_empty()); // No deadlock yet, just a wait relationship

        // Unregister the wait
        detector
            .unregister_wait(pid1.clone(), pid2.clone())
            .await
            .unwrap();

        // Verify the wait is unregistered - should still have no deadlocks
        let deadlocked_after = detector.find_deadlocked_transactions().await;
        assert!(deadlocked_after.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_deadlockdetector_resolution_logic() {
        // Test deadlock resolution finds transactions in cycle
        let detector = DeadlockDetector::new();
        let pid1 = cuid2::cuid();
        let pid2 = cuid2::cuid();
        let pid3 = cuid2::cuid();

        detector
            .register_wait(pid1.clone(), pid2.clone())
            .await
            .unwrap();
        detector
            .register_wait(pid2.clone(), pid3.clone())
            .await
            .unwrap();
        detector
            .register_wait(pid3.clone(), pid1.clone())
            .await
            .unwrap();

        let deadlocked = detector.find_deadlocked_transactions().await;
        assert!(!deadlocked.is_empty());

        for pid in deadlocked {
            assert!(pid == pid1 || pid == pid2 || pid == pid3);
        }

        detector
            .unregister_wait(pid1.clone(), pid2.clone())
            .await
            .unwrap();
        detector
            .unregister_wait(pid2.clone(), pid3.clone())
            .await
            .unwrap();
        detector
            .unregister_wait(pid3.clone(), pid1.clone())
            .await
            .unwrap();
    }

    #[tokio::test]
    #[serial]
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
            .acquire_lock(
                &path,
                LockStrategy::Exclusive,
                Some(Duration::from_millis(100)),
            )
            .await;
        assert!(matches!(
            result,
            Err(crate::error::SentinelError::LockTimeout { .. })
        ));

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
    #[serial]
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
    #[serial]
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
    #[serial]
    async fn test_lockmanager_stats_basic() {
        let manager = Arc::new(FileLockManager::new());
        let stats = manager.get_stats().await;

        assert_eq!(stats.active_locks, 0);
        assert_eq!(stats.total_pending_requests, 0);
        assert_eq!(stats.paths_with_pending_requests, 0);
        assert!(stats.queue_lengths.is_empty());
        assert_eq!(stats.lock_acquisitions, 0);
        assert_eq!(stats.lock_contentions, 0);
        assert_eq!(stats.failed_acquisitions, 0);
    }

    #[tokio::test]
    #[serial]
    async fn test_lockmanager_stats_after_acquisition() {
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_stats_acquisition.json");

        // Acquire a lock
        let _guard = manager
            .acquire_lock(&path, LockStrategy::Exclusive, None)
            .await
            .unwrap();

        let stats = manager.get_stats().await;

        assert_eq!(stats.active_locks, 1);
        assert_eq!(stats.total_pending_requests, 0);
        assert_eq!(stats.paths_with_pending_requests, 0);
        assert!(stats.queue_lengths.is_empty());
        assert_eq!(stats.lock_acquisitions, 1);
        assert_eq!(stats.lock_contentions, 0);
        assert_eq!(stats.failed_acquisitions, 0);
    }

    #[tokio::test]
    #[serial]
    async fn test_lockmanager_stats_with_contention() {
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_stats_contention.json");

        // Hold exclusive lock
        let guard = manager
            .acquire_lock(&path, LockStrategy::Exclusive, None)
            .await
            .unwrap();

        // Create contention by trying to acquire the same lock with timeout
        let timeout_result = manager
            .acquire_lock(
                &path,
                LockStrategy::Exclusive,
                Some(Duration::from_millis(50)),
            )
            .await;

        // Should fail due to timeout (enqueues then times out)
        assert!(timeout_result.is_err());

        let stats = manager.get_stats().await;

        let stats = manager.get_stats().await;

        assert_eq!(stats.active_locks, 1);
        assert_eq!(stats.total_pending_requests, 0); // Request was removed after timeout
        assert_eq!(stats.paths_with_pending_requests, 0);
        assert!(stats.queue_lengths.is_empty());
        assert_eq!(stats.lock_acquisitions, 1);
        assert_eq!(stats.lock_contentions, 1); // Request was enqueued before timing out
        assert_eq!(stats.failed_acquisitions, 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_lockmanager_stats_with_queue() {
        let manager = Arc::new(FileLockManager::new());
        let path = PathBuf::from("/tmp/test_stats_queue.json");

        // Hold exclusive lock
        let guard = manager
            .acquire_lock(&path, LockStrategy::Exclusive, None)
            .await
            .unwrap();

        // Start a background task that will queue for the lock
        let manager_clone = Arc::clone(&manager);
        let path_clone = path.clone();
        let handle = tokio::spawn(async move {
            let _queued_guard = manager_clone
                .acquire_lock(
                    &path_clone,
                    LockStrategy::Exclusive,
                    Some(Duration::from_millis(200)),
                )
                .await;
            // Guard will be dropped here
        });

        // Give time for the queued request to be enqueued
        tokio::time::sleep(Duration::from_millis(100)).await;

        let stats = manager.get_stats().await;

        // Should have 1 queued request
        assert_eq!(stats.active_locks, 1);
        assert_eq!(stats.total_pending_requests, 1);
        assert_eq!(stats.paths_with_pending_requests, 1);
        assert_eq!(stats.queue_lengths.len(), 1);
        assert_eq!(
            stats.queue_lengths.get(&path.to_string_lossy().to_string()),
            Some(&1)
        );
        assert_eq!(stats.lock_acquisitions, 1);
        assert_eq!(stats.lock_contentions, 1); // One request was queued

        // Release lock, allowing queued request to proceed
        drop(guard);
        let _ = handle.await; // Wait for background task to complete

        // Give a small delay to ensure all cleanup is done
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Check final stats
        let final_stats = manager.get_stats().await;
        assert_eq!(final_stats.active_locks, 0); // Lock released
        assert_eq!(final_stats.total_pending_requests, 0);
        assert_eq!(final_stats.paths_with_pending_requests, 0);
        assert!(final_stats.queue_lengths.is_empty());
        assert_eq!(final_stats.lock_acquisitions, 2); // Both acquisitions succeeded
        assert_eq!(final_stats.lock_contentions, 1);
        assert_eq!(final_stats.failed_acquisitions, 0);
    }

    #[tokio::test]
    #[serial]
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
        assert!(!manager.is_locked(&path).await);
    }

    #[tokio::test]
    #[serial]
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
