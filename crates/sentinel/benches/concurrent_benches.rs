use std::{hint::black_box, sync::Arc};

use criterion::{criterion_group, criterion_main, Criterion};
use sentinel_dbms::{Collection, Store};
use serde_json::json;
use tempfile::tempdir;
use tokio::runtime::Runtime;

async fn setup_concurrent_collection(count: usize) -> (Arc<Collection>, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let store = Store::new(temp_dir.path(), None).await.unwrap();
    let collection = Arc::new(store.collection("concurrent_test").await.unwrap());

    // Pre-populate with some data
    for i in 0 .. count {
        let doc = json!({"id": i, "value": i * 10, "thread": "setup"});
        collection.insert(&format!("doc_{}", i), doc).await.unwrap();
    }

    (collection, temp_dir)
}

fn bench_concurrent_reads(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("concurrent_reads_10_threads", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_concurrent_collection(1000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let mut handles = vec![];

                    for thread_id in 0 .. 10 {
                        let coll = Arc::clone(&collection);
                        let handle = tokio::spawn(async move {
                            for i in (thread_id * 100) .. ((thread_id + 1) * 100) {
                                let doc = coll.get(&format!("doc_{}", i)).await.unwrap();
                                black_box(doc);
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.unwrap();
                    }
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_concurrent_writes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("concurrent_writes_5_threads", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_concurrent_collection(100).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let mut handles = vec![];

                    for thread_id in 0 .. 5 {
                        let coll = Arc::clone(&collection);
                        let handle = tokio::spawn(async move {
                            for i in 0 .. 50 {
                                let doc_id = format!("bulk_{}_{}", thread_id, i);
                                let doc = json!({
                                    "id": format!("{}_{}", thread_id, i),
                                    "value": i,
                                    "thread": thread_id
                                });
                                coll.insert(&doc_id, doc).await.unwrap();
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.unwrap();
                    }
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_mixed_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("mixed_concurrent_operations", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_concurrent_collection(500).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let mut handles = vec![];

                    // Reader threads
                    for thread_id in 0 .. 3 {
                        let coll = Arc::clone(&collection);
                        let handle = tokio::spawn(async move {
                            let start = 200 + thread_id * 100;
                            let end = start + 100;
                            for i in start .. end {
                                let doc_id = format!("doc_{}", i);
                                let doc = coll.get(&doc_id).await.unwrap();
                                black_box(doc);
                            }
                        });
                        handles.push(handle);
                    }

                    // Writer threads
                    for thread_id in 0 .. 2 {
                        let coll = Arc::clone(&collection);
                        let handle = tokio::spawn(async move {
                            for i in 0 .. 100 {
                                let doc_id = format!("mixed_doc_{}_{}", thread_id, i);
                                let doc = json!({
                                    "id": format!("mixed_{}_{}", thread_id, i),
                                    "value": i,
                                    "thread": thread_id,
                                    "operation": "write"
                                });
                                coll.insert(&doc_id, doc).await.unwrap();
                            }
                        });
                        handles.push(handle);
                    }

                    // Updater threads
                    for thread_id in 0 .. 2 {
                        let coll = Arc::clone(&collection);
                        let handle = tokio::spawn(async move {
                            let start = thread_id * 50;
                            let end = start + 50;
                            for i in start .. end {
                                let doc_id = format!("doc_{}", i);
                                let doc = json!({
                                    "id": i,
                                    "value": i * 100,
                                    "thread": thread_id,
                                    "operation": "update"
                                });
                                coll.update(&doc_id, doc).await.unwrap();
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.unwrap();
                    }
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_concurrent_bulk_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("concurrent_bulk_operations", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_concurrent_collection(100).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let mut handles = vec![];

                    for thread_id in 0 .. 3 {
                        let coll = Arc::clone(&collection);
                        let handle = tokio::spawn(async move {
                            // Bulk insert - use individual inserts for simplicity
                            for i in 0 .. 50 {
                                let doc_id = format!("bulk_{}_{}", thread_id, i);
                                let doc = json!({
                                    "id": format!("{}_{}", thread_id, i),
                                    "value": i,
                                    "thread": thread_id
                                });
                                coll.insert(&doc_id, doc).await.unwrap();
                            }

                            // Bulk get
                            let ids: Vec<String> = (0 .. 25).map(|i| format!("doc_{}", i % 100)).collect();
                            let str_ids: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
                            let docs = coll.get_many(&str_ids).await.unwrap();
                            black_box(docs.len());
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.unwrap();
                    }
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    concurrent_benches,
    bench_concurrent_reads,
    bench_concurrent_writes,
    bench_mixed_concurrent_operations,
    bench_concurrent_bulk_operations
);
criterion_main!(concurrent_benches);
