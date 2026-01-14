use serde_json::Value;

/// Trait for hash functions
pub trait HashFunction {
    fn hash_data(data: &Value) -> String;
}

/// Blake3 implementation
pub struct Blake3Hasher;

impl HashFunction for Blake3Hasher {
    fn hash_data(data: &Value) -> String {
        let json_str = serde_json::to_string(data).expect("Failed to serialize data");
        blake3::hash(json_str.as_bytes()).to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_hash() {
        let data = serde_json::json!({"key": "value", "number": 42});
        let hash = Blake3Hasher::hash_data(&data);
        assert_eq!(hash.len(), 64);
        let hash2 = Blake3Hasher::hash_data(&data);
        assert_eq!(hash, hash2);
    }
}
