pub mod hash;
pub mod key;
pub mod sign;

use ed25519_dalek::SigningKey;
pub use hash::{Blake3Hasher, HashFunction};
pub use key::KeyManager;
// Convenience functions using default implementations
use serde_json::Value;
pub use sign::{Ed25519Signer, SignatureAlgorithm};

/// Computes the Blake3 hash of the given JSON data.
pub fn hash_data(data: &Value) -> String { Blake3Hasher::hash_data(data) }

/// Signs the given hash using Ed25519.
pub fn sign_hash(hash: &str, private_key: &SigningKey) -> Result<String, Box<dyn std::error::Error>> {
    Ed25519Signer::sign_hash(hash, private_key)
}

/// Verifies the signature of the given hash using Ed25519.
pub fn verify_signature(
    hash: &str,
    signature: &str,
    public_key: &ed25519_dalek::VerifyingKey,
) -> Result<bool, Box<dyn std::error::Error>> {
    Ed25519Signer::verify_signature(hash, signature, public_key)
}

#[cfg(test)]
mod tests {
    use rand::random;

    use super::*;

    #[test]
    fn test_hash_data() {
        let data = serde_json::json!({"key": "value", "number": 42});
        let hash = hash_data(&data);
        assert_eq!(hash.len(), 64);
        let hash2 = hash_data(&data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_sign_and_verify_hash() {
        let secret: [u8; 32] = random();
        let private_key = SigningKey::from_bytes(&secret);
        let public_key = private_key.verifying_key();

        let hash = "some_hash_value";
        let signature = sign_hash(hash, &private_key).unwrap();

        let is_valid = verify_signature(hash, &signature, &public_key).unwrap();
        assert!(is_valid);

        let is_valid_wrong = verify_signature("wrong_hash", &signature, &public_key).unwrap();
        assert!(!is_valid_wrong);
    }
}
