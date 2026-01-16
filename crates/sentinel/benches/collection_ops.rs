use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use sentinel_dbms::{Collection, Store};
use serde_json::json;
use tempfile::tempdir;

async fn setup_collection() -> (Collection, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let store = Store::new(temp_dir.path(), None).await.unwrap();
    let collection = store.collection("bench_collection").await.unwrap();
    (collection, temp_dir)
}

fn bench_insert(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_insert", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (collection, _temp_dir) = setup_collection().await;
                let doc = json!({"name": "test", "value": black_box(42)});
                collection.insert("test-id", doc).await.unwrap();
            });
        })
    });
}

fn bench_get(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_get", |b| {
        b.iter_batched(
            || {
                rt.block_on(async {
                    let (collection, temp_dir) = setup_collection().await;
                    let doc = json!({"name": "test", "value": black_box(42)});
                    collection.insert("test-id", doc).await.unwrap();
                    (collection, temp_dir)
                })
            },
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    black_box(collection.get("test-id").await.unwrap());
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_update(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_update", |b| {
        b.iter_batched(
            || {
                rt.block_on(async {
                    let (collection, temp_dir) = setup_collection().await;
                    let doc = json!({"name": "test", "value": black_box(42)});
                    collection.insert("test-id", doc).await.unwrap();
                    (collection, temp_dir)
                })
            },
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let doc = json!({"name": "updated", "value": black_box(43)});
                    collection.update("test-id", doc).await.unwrap();
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_delete(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_delete", |b| {
        b.iter_batched(
            || {
                rt.block_on(async {
                    let (collection, temp_dir) = setup_collection().await;
                    let doc = json!({"name": "test", "value": black_box(42)});
                    collection.insert("test-id", doc).await.unwrap();
                    (collection, temp_dir)
                })
            },
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    collection.delete("test-id").await.unwrap();
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_list(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_list", |b| {
        b.iter_batched(
            || {
                rt.block_on(async {
                    let (collection, temp_dir) = setup_collection().await;
                    for i in 0..10 {
                        let doc = json!({"name": format!("test{}", i), "value": black_box(i)});
                        collection.insert(&format!("test-id-{}", i), doc).await.unwrap();
                    }
                    (collection, temp_dir)
                })
            },
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    black_box(collection.list().await.unwrap());
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_insert, bench_get, bench_update, bench_delete, bench_list);
criterion_main!(benches);
