use std::hint::black_box;

use criterion::{async_executor::FuturesExecutor, criterion_group, criterion_main, Criterion};
use futures::TryStreamExt;
use sentinel_crypto::{decrypt_data, encrypt_data, hash_data, verify_signature, SigningKeyManager};
use sentinel_dbms::{Collection, Filter, Operator, QueryBuilder, Store};
use serde_json::json;
use tempfile::tempdir;

// Fuzzy testing for crypto operations with malformed data
fn bench_fuzzy_crypto_malformed_json(c: &mut Criterion) {
    c.bench_function("fuzzy_crypto_malformed_json", |b| {
        b.iter(|| {
            // Test with various malformed JSON that could cause issues
            let malformed_cases = vec![
                json!(null),
                json!(""),
                json!([]),
                json!({}),
                json!({"key": "\u{0000}\u{0001}\u{0002}"}), // null bytes
                json!({"deeply": {"nested": {"structure": {"with": {"many": {"levels": "value"}}}}}}),
                json!({"array": (0..10000).map(|i| format!("item_{}", i)).collect::<Vec<_>>()}), // large array
                json!({"string": "a".repeat(100000)}),                                           // very large string
            ];

            for case in malformed_cases {
                let _ = black_box(hash_data(&case));
            }
        })
    });
}

fn bench_fuzzy_crypto_invalid_keys(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    c.bench_function("fuzzy_crypto_invalid_keys", |b| {
        b.to_async(FuturesExecutor).iter(|| {
            async {
                let data = json!({"test": "data"});
                let hash = hash_data(&data).await.unwrap();

                let key = SigningKeyManager::generate_key();

                // Test with invalid signatures
                let invalid_sigs = vec![
                    "".to_string(),
                    "invalid_hex".to_string(),
                    "deadbeef".to_string(),
                    "00".repeat(100), // too long
                    "gg".to_string(), // invalid hex chars
                ];

                for sig in invalid_sigs {
                    let _ = black_box(verify_signature(&hash, &sig, &key.verifying_key()).await);
                }
            }
        })
    });
}

fn bench_fuzzy_crypto_edge_case_data(c: &mut Criterion) {
    c.bench_function("fuzzy_crypto_edge_case_data", |b| {
        b.to_async(FuturesExecutor).iter(|| {
            async {
                let key = [0u8; 32];

                let edge_cases = vec![
                    vec![],                               // empty
                    vec![0u8],                            // single byte
                    vec![0u8; 1],                         // one zero byte
                    vec![255u8; 100],                     // all max bytes
                    (0u8 ..= 255u8).collect::<Vec<u8>>(), // all byte values
                    vec![0u8; 1000000],                   // 1MB of zeros
                ];

                for data in edge_cases {
                    let _ = black_box(encrypt_data(&data, &key).await);
                }
            }
        })
    });
}

fn bench_fuzzy_crypto_corrupted_ciphertext(c: &mut Criterion) {
    let key = [0u8; 32];
    let data = b"test data";
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let encrypted = rt.block_on(encrypt_data(data, &key)).unwrap();

    c.bench_function("fuzzy_crypto_corrupted_ciphertext", |b| {
        b.iter(|| {
            // Test decryption with various corrupted versions of the ciphertext
            let mut corrupted = encrypted.clone();
            if corrupted.len() > 10 {
                // Flip bits in different positions
                for i in 0 .. corrupted.len().min(20) {
                    let chars: Vec<char> = corrupted.chars().collect();
                    if i < chars.len() {
                        let mut flipped = chars[i] as u8 ^ 0xff;
                        if flipped.is_ascii() {
                            corrupted =
                                corrupted[.. i].to_string() + &(flipped as char).to_string() + &corrupted[i + 1 ..];
                            let _ = black_box(decrypt_data(&corrupted, &key));
                            corrupted = encrypted.clone(); // restore
                        }
                    }
                }

                // Truncate at different lengths
                for len in (1 .. corrupted.len().min(50)).step_by(5) {
                    let truncated = corrupted[.. len].to_string();
                    let _ = black_box(decrypt_data(&truncated, &key));
                }

                // Append garbage
                for extra_len in [1, 10, 100] {
                    let mut extended = corrupted.clone();
                    extended.push_str(&"x".repeat(extra_len));
                    let _ = black_box(decrypt_data(&extended, &key));
                }
            }
        })
    });
}

// Fuzzy testing for database operations
async fn setup_fuzzy_collection() -> (Collection, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let store = Store::new(temp_dir.path(), None).await.unwrap();
    let collection = store.collection("fuzzy_test").await.unwrap();

    // Insert some valid data
    for i in 0 .. 100 {
        let doc = json!({"id": i, "value": i, "valid": true});
        collection.insert(&format!("doc_{}", i), doc).await.unwrap();
    }

    (collection, temp_dir)
}

fn bench_fuzzy_db_malformed_queries(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("fuzzy_db_malformed_queries", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_fuzzy_collection().await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    // Test with various malformed query filters
                    let malformed_filters = vec![
                        Filter::Equals("field".to_string(), json!(null)),
                        Filter::Equals("field".to_string(), json!("")),
                        Filter::Equals("field".to_string(), json!([])),
                        Filter::GreaterThan("field".to_string(), json!(null)),
                        Filter::Contains("field".to_string(), "".to_string()),
                        Filter::In("field".to_string(), vec![]),
                        Filter::Exists("".to_string(), true), // empty field name
                    ];

                    for filter in malformed_filters {
                        use sentinel_dbms::Query;
                        let query = Query {
                            filters:    vec![filter],
                            sort:       None,
                            limit:      None,
                            offset:     None,
                            projection: None,
                        };
                        let _ = black_box(collection.query(query).await);
                    }
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_fuzzy_db_edge_case_documents(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("fuzzy_db_edge_case_documents", |b| {
        b.iter_batched(
            || {
                rt.block_on(async {
                    let (collection, temp_dir) = setup_fuzzy_collection().await;
                    // Insert edge case documents
                    let edge_cases = vec![
                        ("empty", json!({})),
                        ("null_value", json!({"field": null})),
                        ("empty_string", json!({"field": ""})),
                        ("empty_array", json!({"field": []})),
                        ("nested_empty", json!({"nested": {}})),
                        (
                            "very_nested",
                            json!({"a": {"b": {"c": {"d": {"e": "deep"}}}}}),
                        ),
                        (
                            "large_numbers",
                            json!({"big": 999999999999999i64, "small": -999999999999999i64}),
                        ),
                        ("special_chars", json!({"field": "!@#$%^&*()"})),
                        ("unicode", json!({"field": "🚀🔥💯"})),
                        ("long_string", json!({"field": "a".repeat(10000)})),
                        (
                            "many_fields",
                            json!((0 .. 100)
                                .map(|i| (format!("field_{}", i), json!(i)))
                                .collect::<serde_json::Map<_, _>>()),
                        ),
                    ];

                    for (id, doc) in edge_cases {
                        collection.insert(id, doc).await.unwrap();
                    }

                    (collection, temp_dir)
                })
            },
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    // Try to query these edge case documents
                    let query = QueryBuilder::new()
                        .filter("field", Operator::Exists, json!(true))
                        .limit(50)
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

fn bench_fuzzy_db_invalid_ids(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("fuzzy_db_invalid_ids", |b| {
        b.iter_batched(
            || rt.block_on(async { setup_fuzzy_collection().await }),
            |(collection, _temp_dir)| {
                rt.block_on(async move {
                    // Test with various invalid document IDs
                    let long_id = "very_long_id_".repeat(1000);
                    let invalid_ids = vec![
                        "", // empty
                        "id with spaces",
                        "id/with/slashes",
                        "id\\with\\backslashes",
                        "id\nwith\nnewlines",
                        "id\twith\ttabs",
                        "id\u{0000}with\u{0000}nulls", // null bytes
                        &long_id,                      // very long
                        "../escape_attempt",           // path traversal
                        "id:with:colons",
                        "id;with;semicolons",
                    ];

                    for id in invalid_ids {
                        let _ = black_box(collection.get(id).await);
                        let doc = json!({"test": "data"});
                        let _ = black_box(collection.insert(id, doc).await);
                    }
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    fuzzy_benches,
    bench_fuzzy_crypto_malformed_json,
    bench_fuzzy_crypto_invalid_keys,
    bench_fuzzy_crypto_edge_case_data,
    bench_fuzzy_crypto_corrupted_ciphertext,
    bench_fuzzy_db_malformed_queries,
    bench_fuzzy_db_edge_case_documents,
    bench_fuzzy_db_invalid_ids
);
criterion_main!(fuzzy_benches);
