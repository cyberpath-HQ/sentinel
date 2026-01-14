use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use signature::{Signer, Verifier};

/// Trait for signature algorithms
pub trait SignatureAlgorithm {
    type SigningKey;
    type VerifyingKey;
    type Signature;

    fn sign_hash(hash: &str, private_key: &Self::SigningKey) -> Result<String, Box<dyn std::error::Error>>;
    fn verify_signature(
        hash: &str,
        signature: &str,
        public_key: &Self::VerifyingKey,
    ) -> Result<bool, Box<dyn std::error::Error>>;
}

/// Ed25519 implementation
pub struct Ed25519Signer;

impl SignatureAlgorithm for Ed25519Signer {
    type Signature = Signature;
    type SigningKey = SigningKey;
    type VerifyingKey = VerifyingKey;

    fn sign_hash(hash: &str, private_key: &SigningKey) -> Result<String, Box<dyn std::error::Error>> {
        let signature = private_key.sign(hash.as_bytes());
        Ok(hex::encode(signature.to_bytes()))
    }

    fn verify_signature(
        hash: &str,
        signature: &str,
        public_key: &VerifyingKey,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let sig_bytes = hex::decode(signature)?;
        let sig_array: [u8; 64] = sig_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid signature length")?;
        let sig = Signature::from_bytes(&sig_array);
        Ok(public_key.verify(hash.as_bytes(), &sig).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use rand::random;

    use super::*;

    #[test]
    fn test_ed25519_sign_verify() {
        let secret: [u8; 32] = random();
        let private_key = SigningKey::from_bytes(&secret);
        let public_key = private_key.verifying_key();

        let hash = "some_hash_value";
        let signature = Ed25519Signer::sign_hash(hash, &private_key).unwrap();

        let is_valid = Ed25519Signer::verify_signature(hash, &signature, &public_key).unwrap();
        assert!(is_valid);

        let is_valid_wrong = Ed25519Signer::verify_signature("wrong", &signature, &public_key).unwrap();
        assert!(!is_valid_wrong);
    }
}
