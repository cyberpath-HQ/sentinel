use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use futures::TryStreamExt;
use sentinel_dbms::{Aggregation, Collection, Filter, Operator, Query, QueryBuilder, SortOrder, Store};
use serde_json::{json, Value};
use tempfile::tempdir;

async fn setup_collection() -> (Collection, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let store = Store::new(temp_dir.path(), None).await.unwrap();
    let collection = store.collection("bench_collection").await.unwrap();
    (collection, temp_dir)
}

async fn setup_collection_with_data(count: usize) -> (Collection, tempfile::TempDir) {
    let (collection, temp_dir) = setup_collection().await;

    // Insert test data
    for i in 0 .. count {
        let doc = json!({
            "id": i,
            "name": format!("document_{}", i),
            "value": i * 10,
            "category": format!("cat_{}", i % 10),
            "active": i % 2 == 0,
            "tags": vec![format!("tag_{}", i % 5), format!("tag_{}", (i + 1) % 5)],
            "nested": {
                "field": i,
                "data": format!("nested_data_{}", i)
            }
        });
        collection.insert(&format!("doc_{}", i), doc).await.unwrap();
    }

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
                    for i in 0 .. 10 {
                        let doc = json!({"name": format!("test{}", i), "value": black_box(i)});
                        collection
                            .insert(&format!("test-id-{}", i), doc)
                            .await
                            .unwrap();
                    }
                    (collection, temp_dir)
                })
            },
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    black_box(collection.list().try_collect::<Vec<_>>().await.unwrap());
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_count(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_count", |b| {
        b.iter_batched(
            || {
                rt.block_on(async {
                    let (collection, temp_dir) = setup_collection_with_data(100).await;
                    (collection, temp_dir)
                })
            },
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    black_box(collection.count().await.unwrap());
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_bulk_insert(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_bulk_insert_10", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_collection().await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let docs: Vec<(String, Value)> = (0 .. 10)
                        .map(|i| {
                            (
                                format!("bulk_doc_{}", i),
                                json!({"name": format!("bulk{}", i), "value": i}),
                            )
                        })
                        .collect();
                    let str_docs: Vec<(&str, Value)> = docs
                        .iter()
                        .map(|(id, data)| (id.as_str(), data.clone()))
                        .collect();
                    black_box(collection.bulk_insert(str_docs).await.unwrap());
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_get_many(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_get_many_10", |b| {
        b.iter_batched(
            || {
                rt.block_on(async {
                    let (collection, temp_dir) = setup_collection_with_data(20).await;
                    (collection, temp_dir)
                })
            },
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let ids: Vec<String> = (0 .. 10).map(|i| format!("doc_{}", i)).collect();
                    let str_ids: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
                    black_box(collection.get_many(&str_ids).await.unwrap());
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_upsert(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_upsert", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_collection().await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let doc = json!({"name": "upsert_test", "value": black_box(42)});
                    // First upsert should insert
                    black_box(collection.upsert("upsert_id", doc.clone()).await.unwrap());
                    // Second upsert should update
                    let updated_doc = json!({"name": "upsert_test", "value": black_box(43)});
                    black_box(collection.upsert("upsert_id", updated_doc).await.unwrap());
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_query_simple(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_query_simple", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_collection_with_data(1000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let query = QueryBuilder::new()
                        .filter("active", Operator::Equals, json!(true))
                        .build();
                    let result = black_box(collection.query(query).await.unwrap());
                    // Consume the stream
                    result.documents.try_collect::<Vec<_>>().await.unwrap();
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_query_with_sort(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_query_with_sort", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_collection_with_data(1000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let query = QueryBuilder::new()
                        .filter("value", Operator::GreaterThan, json!(500))
                        .sort("value", SortOrder::Descending)
                        .limit(500)
                        .build();
                    let result = black_box(collection.query(query).await.unwrap());
                    result.documents.try_collect::<Vec<_>>().await.unwrap();
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_query_complex(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_query_complex", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_collection_with_data(1000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let complex_filter = Filter::And(
                        Box::new(Filter::Equals("active".to_string(), json!(true))),
                        Box::new(Filter::GreaterThan("value".to_string(), json!(200))),
                    );
                    let query = Query {
                        filters:    vec![complex_filter],
                        sort:       Some(("name".to_string(), SortOrder::Ascending)),
                        limit:      Some(25),
                        offset:     Some(10),
                        projection: None,
                    };
                    let result = black_box(collection.query(query).await.unwrap());
                    result.documents.try_collect::<Vec<_>>().await.unwrap();
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_aggregate_count(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_aggregate_count", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_collection_with_data(1000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let filters = vec![Filter::Equals("active".to_string(), json!(true))];
                    black_box(
                        collection
                            .aggregate(filters, Aggregation::Count)
                            .await
                            .unwrap(),
                    );
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_aggregate_sum(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_aggregate_sum", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_collection_with_data(1000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let filters = vec![Filter::GreaterThan("value".to_string(), json!(100))];
                    black_box(
                        collection
                            .aggregate(filters, Aggregation::Sum("value".to_string()))
                            .await
                            .unwrap(),
                    );
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_aggregate_avg(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("collection_aggregate_avg", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_collection_with_data(1000).await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    let filters = vec![Filter::LessThan("value".to_string(), json!(500))];
                    black_box(
                        collection
                            .aggregate(filters, Aggregation::Avg("value".to_string()))
                            .await
                            .unwrap(),
                    );
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    benches,
    bench_insert,
    bench_get,
    bench_update,
    bench_delete,
    bench_list,
    bench_count,
    bench_bulk_insert,
    bench_get_many,
    bench_upsert,
    bench_query_simple,
    bench_query_with_sort,
    bench_query_complex,
    bench_aggregate_count,
    bench_aggregate_sum,
    bench_aggregate_avg
);
criterion_main!(benches);
