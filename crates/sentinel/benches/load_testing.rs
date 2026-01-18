use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use futures::TryStreamExt;
use sentinel_dbms::{Collection, Filter, Operator, QueryBuilder, SortOrder, Store};
use serde_json::{json, Value};
use tempfile::tempdir;

async fn setup_large_collection(count: usize) -> (Collection, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let store = Store::new(temp_dir.path(), None).await.unwrap();
    let collection = store.collection("load_test_collection").await.unwrap();

    // Insert large dataset
    for i in 0 .. count {
        let doc = json!({
            "id": i,
            "user_id": format!("user_{}", i % 1000),
            "product_id": format!("product_{}", i % 100),
            "category": format!("category_{}", i % 10),
            "price": (i % 1000) as f64,
            "quantity": i % 50,
            "timestamp": i * 1000,
            "active": i % 3 != 0,
            "tags": vec![
                format!("tag_{}", i % 20),
                format!("type_{}", i % 5)
            ],
            "metadata": {
                "created_by": format!("system_{}", i % 10),
                "version": i % 100,
                "flags": {
                    "featured": i % 10 == 0,
                    "discounted": i % 5 == 0,
                    "bestseller": i % 20 == 0
                }
            }
        });
        collection
            .insert(&format!("doc_{:06}", i), doc)
            .await
            .unwrap();
    }

    (collection, temp_dir)
}

fn bench_load_test_insert_10k(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("load_test_insert_10k", |b| {
        b.iter(|| {
            rt.block_on(async {
                let temp_dir = tempdir().unwrap();
                let store = Store::new(temp_dir.path(), None).await.unwrap();
                let collection = store.collection("load_test").await.unwrap();

                for i in 0 .. 10000 {
                    let doc = json!({"id": i, "data": format!("data_{}", i)});
                    collection.insert(&format!("doc_{}", i), doc).await.unwrap();
                }
                black_box(temp_dir);
            });
        })
    });
}

fn bench_load_test_query_10k(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("load_test_query_10k", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_large_collection(10000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let query = QueryBuilder::new()
                        .filter("active", Operator::Equals, json!(true))
                        .limit(100)
                        .build();
                    let result = collection.query(query).await.unwrap();
                    let docs = result.documents.try_collect::<Vec<_>>().await.unwrap();
                    black_box(docs.len());
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_load_test_query_with_sort_10k(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("load_test_query_with_sort_10k", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_large_collection(10000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let query = QueryBuilder::new()
                        .filter("price", Operator::GreaterThan, json!(500.0))
                        .sort("price", SortOrder::Descending)
                        .limit(500)
                        .build();
                    let result = collection.query(query).await.unwrap();
                    let docs = result.documents.try_collect::<Vec<_>>().await.unwrap();
                    black_box(docs.len());
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_load_test_bulk_operations_10k(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("load_test_bulk_operations_10k", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_large_collection(10000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    // Bulk get many documents
                    let ids: Vec<String> = (0 .. 1000).map(|i| format!("doc_{:06}", i)).collect();
                    let str_ids: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
                    let docs = collection.get_many(&str_ids).await.unwrap();
                    black_box(docs.len());

                    // Bulk insert new documents
                    let new_docs: Vec<(String, Value)> = (10000 .. 11000)
                        .map(|i| {
                            (
                                format!("new_doc_{}", i),
                                json!({"id": i, "data": format!("new_data_{}", i)}),
                            )
                        })
                        .collect();
                    let str_docs: Vec<(&str, Value)> = new_docs
                        .iter()
                        .map(|(id, data)| (id.as_str(), data.clone()))
                        .collect();
                    collection.bulk_insert(str_docs).await.unwrap();
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_load_test_aggregate_10k(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("load_test_aggregate_10k", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_large_collection(10000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    use sentinel_dbms::Aggregation;

                    // Count active documents
                    let count_filters = vec![Filter::Equals("active".to_string(), json!(true))];
                    let count = collection
                        .aggregate(count_filters, Aggregation::Count)
                        .await
                        .unwrap();

                    // Sum prices for discounted items
                    let sum_filters = vec![Filter::Equals(
                        "metadata.flags.discounted".to_string(),
                        json!(true),
                    )];
                    let sum = collection
                        .aggregate(sum_filters, Aggregation::Sum("price".to_string()))
                        .await
                        .unwrap();

                    black_box((count, sum));
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    load_benches,
    bench_load_test_insert_10k,
    bench_load_test_query_10k,
    bench_load_test_query_with_sort_10k,
    bench_load_test_bulk_operations_10k,
    bench_load_test_aggregate_10k
);
criterion_main!(load_benches);
