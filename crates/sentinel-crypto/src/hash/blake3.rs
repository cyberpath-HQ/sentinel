use serde_json::Value;
use tracing::trace;

use crate::{error::CryptoError, hash_trait::HashFunction};

/// Blake3 hash implementation.
/// Uses the BLAKE3 cryptographic hash function, which provides high performance
/// and security. Supports parallel computation for large inputs.
///
/// Design choice: BLAKE3 was chosen for its speed (faster than SHA-256/512),
/// security (based on BLAKE2), and parallel processing capabilities.
/// It's a rustcrypto crate, preferred over ring for this use case.
pub struct Blake3Hasher;

impl HashFunction for Blake3Hasher {
    fn hash_data(data: &Value) -> Result<String, CryptoError> {
        trace!("Hashing data with Blake3");
        let json_str = serde_json::to_string(data).map_err(CryptoError::from)?;
        let hash = blake3::hash(json_str.as_bytes());
        let hash_str = hash.to_hex().to_string();
        trace!("Blake3 hash computed: {}", hash_str);
        Ok(hash_str)
    }
}

impl crate::hash_trait::private::Sealed for Blake3Hasher {}

#[test]
fn test_blake3_hash() {
    let data = serde_json::json!({"key": "value", "number": 42});
    let hash = Blake3Hasher::hash_data(&data).unwrap();
    assert_eq!(hash.len(), 64);
    let hash2 = Blake3Hasher::hash_data(&data).unwrap();
    assert_eq!(hash, hash2);
}
