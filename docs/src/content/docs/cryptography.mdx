---
title: Cryptography
description: Configure hashing, digital signatures, encryption, and key derivation in Sentinel.
section: Cryptography
order: 20
keywords: ["cryptography", "encryption", "hashing", "signing", "BLAKE3", "Ed25519", "AES", "ChaCha20"]
related: ["document", "store", "crypto-algorithms"]
---

Sentinel includes a comprehensive cryptography module that provides hashing, digital signatures, encryption, and key
derivation. This guide explains the available algorithms, how to configure them, and when to use each one.

## Overview

The Sentinel crypto module (`sentinel-crypto`) provides four categories of cryptographic operations:

1. **Hashing** — Compute content hashes for integrity verification
2. **Signing** — Create and verify digital signatures for tamper evidence
3. **Encryption** — Encrypt and decrypt sensitive data
4. **Key Derivation** — Derive encryption keys from passphrases

Each category has multiple algorithm choices, allowing you to balance security, performance, and compatibility for your
use case.

## Global Configuration

Cryptographic algorithms are configured globally using `CryptoConfig`:

```rust
use sentinel_dbms::{
    CryptoConfig,
    HashAlgorithmChoice,
    SignatureAlgorithmChoice,
    EncryptionAlgorithmChoice,
    KeyDerivationAlgorithmChoice,
    set_global_crypto_config,
};

fn main() {
    let config = CryptoConfig {
        hash_algorithm: HashAlgorithmChoice::Blake3,
        signature_algorithm: SignatureAlgorithmChoice::Ed25519,
        encryption_algorithm: EncryptionAlgorithmChoice::XChaCha20Poly1305,
        key_derivation_algorithm: KeyDerivationAlgorithmChoice::Argon2id,
    };

    // Must be called before any crypto operations
    set_global_crypto_config(config).expect("Config must be set once");
}
```

The configuration can only be set once per process. Attempting to set it again returns an error.

## Default Configuration

If you don't set a configuration, Sentinel uses secure defaults:

| Category       | Default Algorithm  |
| -------------- | ------------------ |
| Hashing        | BLAKE3             |
| Signing        | Ed25519            |
| Encryption     | XChaCha20-Poly1305 |
| Key Derivation | Argon2id           |

These defaults provide excellent security and performance for most use cases.

## Hashing

Sentinel uses hashing to create content fingerprints for integrity verification.

### BLAKE3 (Default)

BLAKE3 is a modern, high-performance cryptographic hash function:

```rust
use sentinel_dbms::hash_data;
use serde_json::json;

let data = json!({"key": "value", "number": 42});
let hash = hash_data(&data)?;

println!("Hash: {}", hash);  // 64-character hex string
```

BLAKE3 provides several advantages over older hash functions. It offers 256-bit security with protection against length
extension attacks. The algorithm supports parallel computation for large inputs, making it significantly faster than
SHA-256 while maintaining equivalent security.

All documents in Sentinel automatically include a BLAKE3 hash of their data field, enabling you to verify integrity at
any time.

## Digital Signatures

Signatures provide tamper-evident storage by cryptographically binding documents to a signing key.

### Ed25519 (Default)

Ed25519 is a modern elliptic curve signature scheme:

```rust
use sentinel_dbms::{sign_hash, verify_signature, SigningKey};

// Generate a signing key
let key = SigningKey::from_bytes(&rand::random::<[u8; 32]>());

// Sign a hash
let hash = "a1b2c3d4...";
let signature = sign_hash(hash, &key)?;

// Verify the signature
let public_key = key.verifying_key();
let is_valid = verify_signature(hash, &signature, &public_key)?;

assert!(is_valid);
```

Ed25519 provides 128-bit security with fast signing and verification operations. Signatures are 64 bytes long, and
private keys are 32 bytes. The algorithm is deterministic, meaning the same message and key always produce the same
signature.

When you create a Store with a passphrase, Sentinel generates an Ed25519 signing key and uses it to sign all documents
automatically.

## Encryption

Sentinel supports three authenticated encryption algorithms for protecting sensitive data.

### XChaCha20-Poly1305 (Default)

The default encryption algorithm provides excellent security with nonce misuse resistance:

```rust
use sentinel_dbms::{encrypt_data, decrypt_data};

let key = [0u8; 32];  // Your 256-bit key
let plaintext = b"sensitive data";

// Encrypt
let ciphertext = encrypt_data(plaintext, &key)?;

// Decrypt
let decrypted = decrypt_data(&ciphertext, &key)?;

assert_eq!(plaintext, decrypted.as_slice());
```

XChaCha20-Poly1305 uses a 24-byte nonce, making it safe even if nonces are randomly generated. The extended nonce makes
accidental nonce reuse extremely unlikely with random nonce generation. The algorithm provides both confidentiality and
authentication.

### AES-256-GCM-SIV

An alternative with hardware acceleration on supporting CPUs:

```rust
use sentinel_dbms::{
    CryptoConfig,
    EncryptionAlgorithmChoice,
    set_global_crypto_config,
};

let config = CryptoConfig {
    encryption_algorithm: EncryptionAlgorithmChoice::Aes256GcmSiv,
    ..Default::default()
};

set_global_crypto_config(config).expect("Config set once");
```

AES-256-GCM-SIV provides nonce misuse resistance with AES hardware acceleration. It's a good choice when hardware AES
acceleration is available and performance is critical.

### Ascon-128

A lightweight cipher suitable for constrained environments:

```rust
use sentinel_dbms::{
    CryptoConfig,
    EncryptionAlgorithmChoice,
    set_global_crypto_config,
};

let config = CryptoConfig {
    encryption_algorithm: EncryptionAlgorithmChoice::Ascon128,
    ..Default::default()
};

set_global_crypto_config(config).expect("Config set once");
```

Ascon was selected as the winner of the NIST Lightweight Cryptography competition. It's designed for
resource-constrained environments like embedded systems and IoT devices, while still providing strong security.

## Key Derivation

Key derivation functions convert passphrases into cryptographic keys suitable for encryption.

### Argon2id (Default)

The default and recommended key derivation function:

```rust
use sentinel_dbms::{derive_key_from_passphrase, derive_key_from_passphrase_with_salt};

// Derive a key with a new random salt
let (salt, key) = derive_key_from_passphrase("my-secret-passphrase")?;

// Re-derive with the same salt
let same_key = derive_key_from_passphrase_with_salt("my-secret-passphrase", &salt)?;

assert_eq!(key, same_key);
```

Argon2id won the Password Hashing Competition and provides excellent protection against both GPU-based and side-channel
attacks. It's memory-hard, making it expensive to attack with specialized hardware.

### PBKDF2

A widely-supported alternative for compatibility:

```rust
use sentinel_dbms::{
    CryptoConfig,
    KeyDerivationAlgorithmChoice,
    set_global_crypto_config,
};

let config = CryptoConfig {
    key_derivation_algorithm: KeyDerivationAlgorithmChoice::Pbkdf2,
    ..Default::default()
};

set_global_crypto_config(config).expect("Config set once");
```

PBKDF2 is supported in nearly every cryptographic library and is suitable when you need interoperability with other
systems. However, it's less resistant to hardware-accelerated attacks than Argon2id.

## Security Considerations

When using Sentinel's cryptography module, keep these security principles in mind.

**Never hardcode passphrases.** Load them from environment variables, secret managers, or secure configuration files.

**Use the defaults unless you have specific requirements.** The default algorithms provide excellent security for most
use cases.

**Understand the trade-offs.** Stronger algorithms may be slower. Choose based on your security requirements and
performance constraints.

**Protect your signing keys.** The passphrase-derived key stored in `.keys/` is encrypted, but the passphrase itself
must remain secret.

**Verify signatures when consuming data.** If documents should be signed, verify signatures before trusting the data.

## Algorithm Comparison

Here's a comparison of the available algorithms to help you choose:

### Encryption Algorithms

| Algorithm          | Security  | Speed       | Nonce Size | Best For                          |
| ------------------ | --------- | ----------- | ---------- | --------------------------------- |
| XChaCha20-Poly1305 | Excellent | Fast        | 24 bytes   | General use (default)             |
| AES-256-GCM-SIV    | Excellent | Very Fast\* | 12 bytes   | Hardware-accelerated environments |
| Ascon-128          | Excellent | Moderate    | 16 bytes   | Constrained devices               |

\*With hardware AES support

### Key Derivation Algorithms

| Algorithm | Security  | Speed | Memory | Best For                                 |
| --------- | --------- | ----- | ------ | ---------------------------------------- |
| Argon2id  | Excellent | Slow  | High   | Security-critical applications (default) |
| PBKDF2    | Good      | Fast  | Low    | Compatibility requirements               |

## Next Steps

Now that you understand Sentinel's cryptography options, explore:

- **[Store](/docs/store)** — How signing integrates with stores
- **[Document](/docs/document)** — How hashing and signatures protect documents
- **[Error Handling](/docs/errors)** — Handling cryptographic errors
