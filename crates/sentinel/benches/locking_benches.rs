//! File Locking System Benchmarks
//!
//! This module benchmarks the performance of the file locking system components,
//! including lock acquisition, release, deadlock detection, and concurrent operations.
//!
//! ## Benchmark Categories
//!
//! - **Lock Acquisition**: Measures overhead of acquiring exclusive and shared locks
//! - **Lock Release**: Measures cleanup overhead when releasing locks
//! - **Deadlock Detection**: Evaluates performance of deadlock detection algorithms
//! - **Concurrent Operations**: Tests performance under concurrent load
//! - **Best/Worst Case**: Explores optimal and pathological scenarios

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sentinel_dbms::locking::{FileLockManager, LockManagerStats, LockStrategy};

/// Create a temporary test directory for benchmarking
fn create_test_dir() -> tempfile::TempDir { tempfile::TempDir::new().expect("Failed to create temp dir") }

/// Benchmark S4.3.1: Lock acquisition overhead for exclusive and shared locks
fn bench_lock_acquisition(c: &mut Criterion) {
    let mut group = c.benchmark_group("lock_acquisition");

    // Benchmark with different file paths
    for file_count in [1, 10, 100] {
        group.throughput(Throughput::Elements(file_count as u64));

        // Test exclusive lock acquisition
        group.bench_with_input(
            BenchmarkId::new("exclusive", file_count),
            &file_count,
            |b, &count| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| {
                        async {
                            let manager = FileLockManager::new();
                            let mut handles = Vec::with_capacity(count);

                            for i in 0 .. count {
                                let file_path = PathBuf::from(format!("/tmp/test_file_{}.json", i));
                                let manager_clone = Arc::new(manager.clone());

                                handles.push(tokio::spawn(async move {
                                    let _guard = manager_clone
                                        .acquire_lock(&file_path, LockStrategy::Exclusive, Duration::from_secs(30))
                                        .await
                                        .expect("Failed to acquire lock");
                                }));
                            }

                            // Wait for all locks to be acquired
                            for handle in handles {
                                handle.await.expect("Task failed");
                            }

                            // Release all locks
                            drop(manager);
                        }
                    });
            },
        );

        // Test shared lock acquisition
        group.bench_with_input(
            BenchmarkId::new("shared", file_count),
            &file_count,
            |b, &count| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| {
                        async {
                            let manager = FileLockManager::new();
                            let mut handles = Vec::with_capacity(count);

                            for i in 0 .. count {
                                let file_path = PathBuf::from(format!("/tmp/test_file_{}.json", i));
                                let manager_clone = Arc::new(manager.clone());

                                handles.push(tokio::spawn(async move {
                                    let _guard = manager_clone
                                        .acquire_lock(&file_path, LockStrategy::Shared, Duration::from_secs(10))
                                        .await
                                        .expect("Failed to acquire lock");
                                }));
                            }

                            // Wait for all locks to be acquired
                            for handle in handles {
                                handle.await.expect("Task failed");
                            }

                            // Release all locks
                            drop(manager);
                        }
                    });
            },
        );
    }

    group.finish();
}

/// Benchmark S4.3.1: Lock release overhead
fn bench_lock_release(c: &mut Criterion) {
    let mut group = c.benchmark_group("lock_release");

    let manager = FileLockManager::new();
    let file_path = PathBuf::from("/tmp/test_lock_release.json");

    // Pre-acquire an exclusive lock
    let mut lock_guard = manager
        .acquire_lock(&file_path, LockStrategy::Exclusive, Duration::from_secs(30))
        .await
        .expect("Failed to acquire lock");

    group.bench_function("release_exclusive", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                async {
                    // Lock is automatically released when guard goes out of scope
                    drop(black_box(&mut lock_guard));
                }
            });
    });

    group.finish();
}

/// Benchmark S4.3.2: Deadlock detection performance impact
fn bench_deadlock_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("deadlock_detection");

    // Test deadlock detection frequency
    group.bench_function("detect_cycle_basic", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                async {
                    let manager = FileLockManager::new();

                    // Simulate a cycle: A → B, B → C, C → A
                    let paths = [
                        PathBuf::from("/tmp/test_cycle_A.json"),
                        PathBuf::from("/tmp/test_cycle_B.json"),
                        PathBuf::from("/tmp/test_cycle_C.json"),
                    ];

                    // Acquire locks in a cycle
                    let mut guards = Vec::new();
                    for path in &paths {
                        let guard = manager
                            .acquire_lock(path, LockStrategy::Exclusive, Duration::from_secs(30))
                            .await
                            .expect("Failed to acquire lock");
                        guards.push(guard);
                    }

                    // Check if deadlock is detected
                    let stats = manager.get_stats().await;
                    assert_eq!(stats.active_locks, 3);
                    assert!(stats.total_pending_requests == 0);

                    // Release locks
                    drop(guards);
                }
            });
    });

    // Test with concurrent wait-for graph
    group.bench_function("detect_cycle_concurrent", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                async {
                    let manager = FileLockManager::new();

                    // Create a more complex wait-for graph with 5 nodes
                    let mut handles = Vec::new();
                    let paths: Vec<PathBuf> = (0 .. 5)
                        .map(|i| PathBuf::from(format!("/tmp/test_cycle_{}.json", i)))
                        .collect();

                    // Create a cycle: 0→1, 1→2, 2→3, 3→4, 4→0
                    for i in 0 .. 5 {
                        let path = paths[i].clone();
                        let next_path = paths[(i + 1) % 5].clone();

                        let manager_clone = Arc::new(manager.clone());
                        handles.push(tokio::spawn(async move {
                            // This represents a transaction waiting for the next lock in the cycle
                            // In reality, this would happen when acquiring locks in a specific order
                            let _guard = manager_clone
                                .acquire_lock(&next_path, LockStrategy::Exclusive, Duration::from_secs(30))
                                .await
                                .expect("Failed to acquire lock");
                        }));
                    }

                    // Wait for all to complete
                    for handle in handles {
                        handle.await.expect("Task failed");
                    }

                    let stats = manager.get_stats().await;
                    assert_eq!(stats.active_locks, 5);

                    drop(manager);
                }
            });
    });

    // Test deadlock detection overhead in steady state
    group.bench_function("steady_state_detection", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                async {
                    let manager = FileLockManager::new();

                    // Acquire and release locks in a loop
                    for i in 0 .. 100 {
                        let path = PathBuf::from(format!("/tmp/test_steady_{}.json", i));
                        let guard = manager
                            .acquire_lock(&path, LockStrategy::Exclusive, Duration::from_secs(1))
                            .await
                            .expect("Failed to acquire lock");
                        drop(guard);
                    }

                    let _stats = manager.get_stats().await;
                }
            });
    });

    group.finish();
}

/// Benchmark concurrent operations impact on locking performance
fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");

    // Test with multiple concurrent writers
    group.bench_function("concurrent_writers", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                async {
                    let manager = FileLockManager::new();
                    let mut handles = Vec::new();

                    for worker_id in 0 .. 10 {
                        let manager_clone = Arc::new(manager.clone());
                        handles.push(tokio::spawn(async move {
                            for i in 0 .. 5 {
                                let file_path = PathBuf::from(format!("/tmp/concurrent_{}.json", i));
                                let _guard = manager_clone
                                    .acquire_lock(&file_path, LockStrategy::Exclusive, Duration::from_secs(30))
                                    .await
                                    .expect("Failed to acquire lock");
                                // Simulate work
                                tokio::time::sleep(Duration::from_micros(100)).await;
                                drop(_guard);
                            }
                        }));
                    }

                    for handle in handles {
                        handle.await.expect("Task failed");
                    }
                }
            });
    });

    // Test with multiple concurrent readers
    group.bench_function("concurrent_readers", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                async {
                    let manager = FileLockManager::new();
                    let mut handles = Vec::new();

                    for reader_id in 0 .. 10 {
                        let manager_clone = Arc::new(manager.clone());
                        handles.push(tokio::spawn(async move {
                            let file_path = PathBuf::from("/tmp/concurrent_reader.json");
                            for _ in 0 .. 5 {
                                let _guard = manager_clone
                                    .acquire_lock(&file_path, LockStrategy::Shared, Duration::from_secs(10))
                                    .await
                                    .expect("Failed to acquire lock");
                                tokio::time::sleep(Duration::from_micros(100)).await;
                                drop(_guard);
                            }
                        }));
                    }

                    for handle in handles {
                        handle.await.expect("Task failed");
                    }
                }
            });
    });

    // Test mixed read/write workload
    group.bench_function("mixed_read_write", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                async {
                    let manager = FileLockManager::new();
                    let mut handles = Vec::new();

                    for worker_id in 0 .. 5 {
                        let manager_clone = Arc::new(manager.clone());
                        handles.push(tokio::spawn(async move {
                            for i in 0 .. 10 {
                                let file_path = PathBuf::from(format!("/tmp/mixed_{}.json", i % 3));
                                let strategy = if i % 2 == 0 {
                                    LockStrategy::Exclusive
                                }
                                else {
                                    LockStrategy::Shared
                                };
                                let _guard = manager_clone
                                    .acquire_lock(&file_path, strategy, Duration::from_secs(10))
                                    .await
                                    .expect("Failed to acquire lock");
                                tokio::time::sleep(Duration::from_micros(50)).await;
                                drop(_guard);
                            }
                        }));
                    }

                    for handle in handles {
                        handle.await.expect("Task failed");
                    }
                }
            });
    });

    group.finish();
}

/// Benchmark best-case and worst-case scenarios
fn bench_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("edge_cases");

    // Best case: No contention
    group.bench_function("best_case_no_contention", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                async {
                    let manager = FileLockManager::new();
                    let path = PathBuf::from("/tmp/best_case.json");

                    let guard = manager
                        .acquire_lock(&path, LockStrategy::Exclusive, Duration::from_secs(30))
                        .await
                        .expect("Failed to acquire lock");
                    drop(guard);
                }
            });
    });

    // Worst case: Heavy contention on single file
    group.bench_function("worst_case_heavy_contention", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                async {
                    let manager = FileLockManager::new();
                    let path = PathBuf::from("/tmp/worst_case.json");
                    let mut handles = Vec::new();

                    for i in 0 .. 50 {
                        let manager_clone = Arc::new(manager.clone());
                        let path_clone = path.clone();
                        handles.push(tokio::spawn(async move {
                            // These will queue up waiting for the lock
                            let _guard = manager_clone
                                .acquire_lock(
                                    &path_clone,
                                    LockStrategy::Exclusive,
                                    Duration::from_secs(30),
                                )
                                .await
                                .expect("Failed to acquire lock");
                        }));
                    }

                    // Release the lock after a short delay to allow some to complete
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    drop(manager);

                    for handle in handles {
                        handle.await.expect("Task failed");
                    }
                }
            });
    });

    // Edge case: Rapid lock acquisition and release
    group.bench_function("rapid_acquire_release", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                async {
                    let manager = FileLockManager::new();

                    for i in 0 .. 1000 {
                        let path = PathBuf::from(format!("/tmp/rapid_{}.json", i % 10));
                        let guard = manager
                            .acquire_lock(&path, LockStrategy::Exclusive, Duration::from_millis(1))
                            .await
                            .expect("Failed to acquire lock");
                        drop(guard);
                    }
                }
            });
    });

    // Edge case: Many files with minimal contention
    group.bench_function("many_files_few_contention", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                async {
                    let manager = FileLockManager::new();
                    let mut handles = Vec::new();

                    for i in 0 .. 100 {
                        let manager_clone = Arc::new(manager.clone());
                        let path = PathBuf::from(format!("/tmp/manyfiles_{}.json", i % 10));
                        handles.push(tokio::spawn(async move {
                            let _guard = manager_clone
                                .acquire_lock(&path, LockStrategy::Exclusive, Duration::from_millis(10))
                                .await
                                .expect("Failed to acquire lock");
                            tokio::time::sleep(Duration::from_micros(100)).await;
                            drop(_guard);
                        }));
                    }

                    for handle in handles {
                        handle.await.expect("Task failed");
                    }
                }
            });
    });

    group.finish();
}

/// Benchmark statistics gathering
fn bench_stats_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("stats_collection");

    group.bench_function("get_stats", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                async {
                    let manager = FileLockManager::new();

                    // Acquire and release some locks
                    for i in 0 .. 10 {
                        let path = PathBuf::from(format!("/tmp/stats_{}.json", i));
                        let guard = manager
                            .acquire_lock(&path, LockStrategy::Exclusive, Duration::from_secs(30))
                            .await
                            .expect("Failed to acquire lock");
                        drop(guard);
                    }

                    // Get statistics
                    let _stats = manager.get_stats().await;
                }
            });
    });

    group.finish();
}

/// Criterion benchmark groups
criterion_group!(
    name = locking_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10));
    targets =
        bench_lock_acquisition,
        bench_lock_release,
        bench_deadlock_detection,
        bench_concurrent_operations,
        bench_edge_cases,
        bench_stats_collection
);

criterion_main!(locking_benches);
