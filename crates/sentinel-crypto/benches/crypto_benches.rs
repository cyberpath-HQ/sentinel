use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use sentinel_crypto::{decrypt_data, derive_key_from_passphrase, encrypt_data, hash_data, sign_hash, verify_signature, SigningKeyManager};
use serde_json::json;

fn bench_hash_data(c: &mut Criterion) {
    let data = json!({"key": "value", "number": 42, "array": [1,2,3,4,5]});

    c.bench_function("hash_data", |b| b.iter(|| hash_data(black_box(&data))));
}

fn bench_sign_hash(c: &mut Criterion) {
    let key = SigningKeyManager::generate_key();
    let hash = "some_hash_value";

    c.bench_function("sign_hash", |b| {
        b.iter(|| sign_hash(black_box(hash), black_box(&key)))
    });
}

fn bench_verify_signature(c: &mut Criterion) {
    let key = SigningKeyManager::generate_key();
    let public_key = key.verifying_key();
    let hash = "some_hash_value";
    let signature = sign_hash(hash, &key).unwrap();

    c.bench_function("verify_signature", |b| {
        b.iter(|| {
            verify_signature(
                black_box(hash),
                black_box(&signature),
                black_box(&public_key),
            )
        })
    });
}

fn bench_generate_key(c: &mut Criterion) {
    c.bench_function("generate_key", |b| {
        b.iter(|| SigningKeyManager::generate_key())
    });
}

fn bench_encrypt_data(c: &mut Criterion) {
    let key = [0u8; 32];
    let data = b"some data to encrypt";

    c.bench_function("encrypt_data", |b| {
        b.iter(|| encrypt_data(black_box(data), black_box(&key)))
    });
}

fn bench_decrypt_data(c: &mut Criterion) {
    let key = [0u8; 32];
    let data = b"some data to encrypt";
    let encrypted = encrypt_data(data, &key).unwrap();

    c.bench_function("decrypt_data", |b| {
        b.iter(|| decrypt_data(black_box(&encrypted), black_box(&key)))
    });
}

fn bench_derive_key_from_passphrase(c: &mut Criterion) {
    let passphrase = "test passphrase";

    c.bench_function("derive_key_from_passphrase", |b| {
        b.iter(|| derive_key_from_passphrase(black_box(passphrase)))
    });
}

criterion_group!(
    benches,
    bench_hash_data,
    bench_sign_hash,
    bench_verify_signature,
    bench_generate_key,
    bench_encrypt_data,
    bench_decrypt_data,
    bench_derive_key_from_passphrase
);
criterion_main!(benches);
