use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sentinel_crypto::{hash_data, sign_hash, verify_signature, KeyManager};
use serde_json::json;

fn bench_hash_data(c: &mut Criterion) {
    let data = json!({"key": "value", "number": 42, "array": [1,2,3,4,5]});

    c.bench_function("hash_data", |b| {
        b.iter(|| hash_data(black_box(&data)))
    });
}

fn bench_sign_hash(c: &mut Criterion) {
    let key = KeyManager::generate_key();
    let hash = "some_hash_value";

    c.bench_function("sign_hash", |b| {
        b.iter(|| sign_hash(black_box(hash), black_box(&key)))
    });
}

fn bench_verify_signature(c: &mut Criterion) {
    let key = KeyManager::generate_key();
    let public_key = key.verifying_key();
    let hash = "some_hash_value";
    let signature = sign_hash(hash, &key).unwrap();

    c.bench_function("verify_signature", |b| {
        b.iter(|| verify_signature(black_box(hash), black_box(&signature), black_box(&public_key)))
    });
}

fn bench_generate_key(c: &mut Criterion) {
    c.bench_function("generate_key", |b| {
        b.iter(|| KeyManager::generate_key())
    });
}

criterion_group!(benches, bench_hash_data, bench_sign_hash, bench_verify_signature, bench_generate_key);
criterion_main!(benches);