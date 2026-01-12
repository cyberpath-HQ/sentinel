use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sentinel::{Collection, Store};
use serde_json::json;
use tempfile::tempdir;

async fn setup_collection() -> (Collection, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let store = Store::new(temp_dir.path()).await.unwrap();
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

fn bench_validate_document_id_valid(c: &mut Criterion) {
    c.bench_function("validate_document_id_valid", |b| {
        b.iter(|| {
            // Best case: simple valid ID
            black_box(Collection::validate_document_id("user-123").unwrap());
        })
    });
}

fn bench_validate_document_id_invalid(c: &mut Criterion) {
    c.bench_function("validate_document_id_invalid", |b| {
        b.iter(|| {
            // Worst case: ID with invalid character
            let _ = black_box(Collection::validate_document_id("user!123").unwrap_err());
        })
    });
}

fn bench_validate_document_id_long(c: &mut Criterion) {
    let long_id = "a".repeat(255);

    c.bench_function("validate_document_id_long", |b| {
        b.iter(|| {
            // Best case with long ID
            black_box(Collection::validate_document_id(&long_id).unwrap());
        })
    });
}

criterion_group!(
    benches,
    bench_insert,
    bench_get,
    bench_update,
    bench_delete,
    bench_validate_document_id_valid,
    bench_validate_document_id_invalid,
    bench_validate_document_id_long
);
criterion_main!(benches);
