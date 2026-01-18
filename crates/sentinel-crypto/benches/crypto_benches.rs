use std::hint::black_box;

use criterion::{async_executor::FuturesExecutor, criterion_group, criterion_main, Criterion};
use sentinel_crypto::{
    decrypt_data,
    derive_key_from_passphrase,
    derive_key_from_passphrase_with_salt,
    encrypt_data,
    hash_data,
    sign_hash,
    verify_signature,
    SigningKeyManager,
};
use serde_json::json;

fn bench_hash_data(c: &mut Criterion) {
    let data = json!({"key": "value", "number": 42, "array": [1,2,3,4,5]});

    c.bench_function("hash_data", |b| {
        b.to_async(FuturesExecutor)
            .iter(|| async { hash_data(black_box(&data)).await })
    });
}

fn bench_sign_hash(c: &mut Criterion) {
    let key = SigningKeyManager::generate_key();
    let hash = "some_hash_value";

    c.bench_function("sign_hash", |b| {
        b.to_async(FuturesExecutor)
            .iter(|| async { sign_hash(black_box(hash), black_box(&key)).await })
    });
}

fn bench_verify_signature(c: &mut Criterion) {
    let key = SigningKeyManager::generate_key();
    let public_key = key.verifying_key();
    let hash = "some_hash_value";
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let signature = rt.block_on(sign_hash(hash, &key)).unwrap();

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
        b.to_async(FuturesExecutor)
            .iter(|| async { encrypt_data(black_box(data), black_box(&key)).await })
    });
}

fn bench_decrypt_data(c: &mut Criterion) {
    let key = [0u8; 32];
    let data = b"some data to encrypt";
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let encrypted = rt.block_on(encrypt_data(data, &key)).unwrap();

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

fn bench_derive_key_from_passphrase_with_salt(c: &mut Criterion) {
    let passphrase = "test passphrase";
    let salt = [0u8; 32];

    c.bench_function("derive_key_from_passphrase_with_salt", |b| {
        b.iter(|| derive_key_from_passphrase_with_salt(black_box(passphrase), black_box(&salt)))
    });
}

fn bench_hash_data_small(c: &mut Criterion) {
    let data = json!({"key": "value"});

    c.bench_function("hash_data_small", |b| {
        b.to_async(FuturesExecutor)
            .iter(|| async { hash_data(black_box(&data)).await })
    });
}

fn bench_hash_data_medium(c: &mut Criterion) {
    let data = json!({
        "key": "value",
        "array": (0..100).map(|i| format!("item_{}", i)).collect::<Vec<_>>(),
        "nested": {
            "deep": {
                "deeper": (0..50).map(|i| json!({"id": i, "data": format!("data_{}", i)})).collect::<Vec<_>>()
            }
        }
    });

    c.bench_function("hash_data_medium", |b| {
        b.to_async(FuturesExecutor)
            .iter(|| async { hash_data(black_box(&data)).await })
    });
}

fn bench_hash_data_large(c: &mut Criterion) {
    let data = json!({
        "key": "value",
        "large_array": (0..100).map(|i| {
            json!({
                "id": i,
                "name": format!("item_{}", i),
                "data": format!("large_data_string_{}_with_lots_of_content_to_make_it_bigger", i).repeat(5),
                "numbers": (0..50).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>()
    });

    c.bench_function("hash_data_large", |b| {
        b.to_async(FuturesExecutor)
            .iter(|| async { hash_data(black_box(&data)).await })
    });
}

fn bench_encrypt_data_small(c: &mut Criterion) {
    let key = [0u8; 32];
    let data = b"small data";

    c.bench_function("encrypt_data_small", |b| {
        b.to_async(FuturesExecutor)
            .iter(|| async { encrypt_data(black_box(data), black_box(&key)).await })
    });
}

fn bench_encrypt_data_medium(c: &mut Criterion) {
    let key = [0u8; 32];
    let data = b"medium sized data that is longer than the small one".repeat(10);

    c.bench_function("encrypt_data_medium", |b| {
        b.to_async(FuturesExecutor)
            .iter(|| async { encrypt_data(black_box(&data), black_box(&key)).await })
    });
}

fn bench_encrypt_data_large(c: &mut Criterion) {
    let key = [0u8; 32];
    let data = b"large data chunk that will be encrypted".repeat(100);

    c.bench_function("encrypt_data_large", |b| {
        b.to_async(FuturesExecutor)
            .iter(|| async { encrypt_data(black_box(&data), black_box(&key)).await })
    });
}

fn bench_decrypt_data_small(c: &mut Criterion) {
    let key = [0u8; 32];
    let data = b"small data";
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let encrypted = rt.block_on(encrypt_data(data, &key)).unwrap();

    c.bench_function("decrypt_data_small", |b| {
        b.iter(|| decrypt_data(black_box(&encrypted), black_box(&key)))
    });
}

fn bench_decrypt_data_medium(c: &mut Criterion) {
    let key = [0u8; 32];
    let data = b"medium sized data that is longer than the small one".repeat(10);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let encrypted = rt.block_on(encrypt_data(&data, &key)).unwrap();

    c.bench_function("decrypt_data_medium", |b| {
        b.iter(|| decrypt_data(black_box(&encrypted), black_box(&key)))
    });
}

fn bench_decrypt_data_large(c: &mut Criterion) {
    let key = [0u8; 32];
    let data = b"large data chunk that will be encrypted".repeat(100);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let encrypted = rt.block_on(encrypt_data(&data, &key)).unwrap();

    c.bench_function("decrypt_data_large", |b| {
        b.iter(|| decrypt_data(black_box(&encrypted), black_box(&key)))
    });
}

criterion_group!(
    benches,
    bench_hash_data,
    bench_hash_data_small,
    bench_hash_data_medium,
    bench_hash_data_large,
    bench_sign_hash,
    bench_verify_signature,
    bench_generate_key,
    bench_encrypt_data,
    bench_encrypt_data_small,
    bench_encrypt_data_medium,
    bench_encrypt_data_large,
    bench_decrypt_data,
    bench_decrypt_data_small,
    bench_decrypt_data_medium,
    bench_decrypt_data_large,
    bench_derive_key_from_passphrase,
    bench_derive_key_from_passphrase_with_salt
);
criterion_main!(benches);
