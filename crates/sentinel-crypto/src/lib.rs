use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use serde_json::Value;
use signature::{Signer, Verifier};

/// Computes the Blake3 hash of the given JSON data.
/// The data is serialized to a canonical JSON string before hashing.
pub fn hash_data(data: &Value) -> String {
    let json_str = serde_json::to_string(data).expect("Failed to serialize data");
    blake3::hash(json_str.as_bytes()).to_hex().to_string()
}

/// Signs the given hash using Ed25519.
/// Returns the signature as a hex-encoded string.
pub fn sign_hash(hash: &str, private_key: &SigningKey) -> Result<String, Box<dyn std::error::Error>> {
    let signature = private_key.sign(hash.as_bytes());
    Ok(hex::encode(signature.to_bytes()))
}

/// Verifies the signature of the given hash using Ed25519.
/// The signature should be hex-encoded.
pub fn verify_signature(
    hash: &str,
    signature: &str,
    public_key: &VerifyingKey,
) -> Result<bool, Box<dyn std::error::Error>> {
    let sig_bytes = hex::decode(signature)?;
    let sig_array: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid signature length")?;
    let signature = Signature::from_bytes(&sig_array);
    Ok(public_key.verify(hash.as_bytes(), &signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_data() {
        let data = serde_json::json!({"key": "value", "number": 42});
        let hash = hash_data(&data);
        assert_eq!(hash.len(), 64); // Blake3 hex is 64 chars
                                    // Hash should be deterministic
        let hash2 = hash_data(&data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_sign_and_verify_hash() {
        let secret: [u8; 32] = rand::random();
        let private_key = SigningKey::from_bytes(&secret);
        let public_key = private_key.verifying_key();

        let hash = "some_hash_value";
        let signature = sign_hash(hash, &private_key).unwrap();

        let is_valid = verify_signature(hash, &signature, &public_key).unwrap();
        assert!(is_valid);

        // Test with wrong hash
        let is_valid_wrong = verify_signature("wrong_hash", &signature, &public_key).unwrap();
        assert!(!is_valid_wrong);
    }
}
