use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sentinel_wal::{EntryType, LogEntry, WalManager};
use serde_json::json;
use tempfile::tempdir;
use cuid2;

/// Benchmark WAL entry serialization
fn bench_log_entry_serialization(c: &mut Criterion) {
    let entry = LogEntry::new(
        EntryType::Insert,
        cuid2::create_id(),
        "users".to_string(),
        "user-123".to_string(),
        Some(json!({"name": "Alice", "email": "alice@example.com"})),
    );

    c.bench_function("log_entry_serialization", |b| {
        b.iter(|| {
            let _bytes = black_box(entry.to_bytes().unwrap());
        });
    });
}

/// Benchmark WAL entry deserialization
fn bench_log_entry_deserialization(c: &mut Criterion) {
    let entry = LogEntry::new(
        EntryType::Insert,
        cuid2::create_id(),
        "users".to_string(),
        "user-123".to_string(),
        Some(json!({"name": "Alice", "email": "alice@example.com"})),
    );
    let bytes = entry.to_bytes().unwrap();

    c.bench_function("log_entry_deserialization", |b| {
        b.iter(|| {
            let _entry = black_box(LogEntry::from_bytes(&bytes).unwrap());
        });
    });
}

/// Benchmark WAL write operations
fn bench_wal_write(c: &mut Criterion) {
    let temp_dir = tempdir().unwrap();
    let wal_path = temp_dir.path().join("bench.wal");
    let wal = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { WalManager::new(wal_path).await.unwrap() });

    let entry = LogEntry::new(
        EntryType::Insert,
        cuid2::create_id(),
        "users".to_string(),
        "user-123".to_string(),
        Some(json!({"name": "Alice", "email": "alice@example.com"})),
    );

    c.bench_function("wal_write_entry", |b| {
        b.iter(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                black_box(wal.write_entry(entry.clone()).await.unwrap());
            });
        });
    });
}

/// Benchmark WAL read operations
fn bench_wal_read(c: &mut Criterion) {
    let temp_dir = tempdir().unwrap();
    let wal_path = temp_dir.path().join("bench.wal");
    let wal = tokio::runtime::Runtime::new().unwrap().block_on(async {
        let wal = WalManager::new(wal_path).await.unwrap();

        // Write some entries
        for i in 0 .. 100 {
            let entry = LogEntry::new(
                EntryType::Insert,
                cuid2::create_id(),
                "users".to_string(),
                format!("user-{}", i),
                Some(json!({"name": format!("User {}", i), "id": i})),
            );
            wal.write_entry(entry).await.unwrap();
        }

        wal
    });

    c.bench_function("wal_read_all_entries", |b| {
        b.iter(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let _entries = black_box(wal.read_all_entries().await.unwrap());
            });
        });
    });
}

criterion_group!(
    benches,
    bench_log_entry_serialization,
    bench_log_entry_deserialization,
    bench_wal_write,
    bench_wal_read
);
criterion_main!(benches);
