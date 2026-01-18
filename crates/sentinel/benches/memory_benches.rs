use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use futures::TryStreamExt;
use sentinel_dbms::{Collection, Operator, QueryBuilder, Store};
use serde_json::json;
use tempfile::tempdir;

async fn setup_memory_test_collection(count: usize) -> (Collection, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let store = Store::new(temp_dir.path(), None).await.unwrap();
    let collection = store.collection("memory_test").await.unwrap();

    // Insert documents with varying sizes
    for i in 0 .. count {
        let doc = json!({
            "id": i,
            "small_field": i,
            "medium_field": format!("medium_data_{}_with_some_content", i),
            "large_field": format!("large_data_{}_with_lots_of_content_to_increase_memory_usage_", i).repeat(10),
            "array_field": (0..100).map(|j| format!("array_item_{}_{}", i, j)).collect::<Vec<_>>(),
            "nested": {
                "level1": {
                    "level2": {
                        "data": format!("nested_data_{}", i),
                        "array": (0..50).collect::<Vec<_>>()
                    }
                }
            }
        });
        collection
            .insert(&format!("mem_doc_{:06}", i), doc)
            .await
            .unwrap();
    }

    (collection, temp_dir)
}

fn bench_memory_large_collection(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("memory_large_collection_10k", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_memory_test_collection(10000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    // Query that loads many documents into memory
                    let query = QueryBuilder::new()
                        .filter("small_field", Operator::GreaterThan, json!(5000))
                        .limit(1000)
                        .build();

                    let result = collection.query(query).await.unwrap();
                    let docs: Vec<_> = result.documents.try_collect().await.unwrap();

                    // Calculate total memory usage roughly
                    let total_size: usize = docs
                        .iter()
                        .map(|doc| serde_json::to_string(doc).unwrap().len())
                        .sum();

                    black_box(total_size);
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_memory_bulk_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("memory_bulk_operations_1k", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_memory_test_collection(1000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    // Bulk get many large documents
                    let ids: Vec<String> = (0 .. 500).map(|i| format!("mem_doc_{:06}", i)).collect();
                    let str_ids: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();

                    let docs = collection.get_many(&str_ids).await.unwrap();
                    let valid_docs: Vec<_> = docs.into_iter().flatten().collect();

                    // Calculate memory usage
                    let total_size: usize = valid_docs
                        .iter()
                        .map(|doc| serde_json::to_string(doc).unwrap().len())
                        .sum();

                    black_box(total_size);
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    memory_benches,
    bench_memory_large_collection,
    bench_memory_bulk_operations
);
criterion_main!(memory_benches);
