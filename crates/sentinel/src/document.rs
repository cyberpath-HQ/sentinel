use chrono::{DateTime, Utc};
use sentinel_crypto::SigningKey;
use sentinel_crypto::{hash_data, sign_hash};
use serde_json::Value;

/// Represents a document in the database.
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Document {
    /// The unique identifier of the document.
    pub(crate) id:         String,
    /// The version of the document, represents the version of the client that created it.
    pub(crate) version:    u32,
    /// The timestamp when the document was created.
    pub(crate) created_at: DateTime<Utc>,
    /// The timestamp when the document was last updated.
    pub(crate) updated_at: DateTime<Utc>,
    /// The hash of the document data.
    pub(crate) hash:       String,
    /// The signature of the document data.
    pub(crate) signature:  String,
    /// The JSON data of the document.
    pub(crate) data:       Value,
}

impl Document {
    /// Creates a new document with the given id, version, and data.
    /// Computes the hash and signature using the provided private key.
    pub fn new(
        id: String,
        version: u32,
        data: Value,
        private_key: &SigningKey,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let now = Utc::now();
        let hash = hash_data(&data);
        let signature = sign_hash(&hash, private_key)?;
        Ok(Self {
            id,
            version,
            created_at: now,
            updated_at: now,
            hash,
            signature,
            data,
        })
    }

    /// Returns the document ID.
    pub fn id(&self) -> &str { &self.id }

    /// Returns the document version.
    pub fn version(&self) -> u32 { self.version }

    /// Returns the creation timestamp.
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }

    /// Returns the last update timestamp.
    pub fn updated_at(&self) -> DateTime<Utc> { self.updated_at }

    /// Returns the hash of the document data.
    pub fn hash(&self) -> &str { &self.hash }

    /// Returns the signature of the document data.
    pub fn signature(&self) -> &str { &self.signature }

    /// Returns a reference to the document data.
    pub fn data(&self) -> &Value { &self.data }

    /// Sets the document data, updates the hash and signature, and refreshes the updated_at
    /// timestamp.
    pub fn set_data(&mut self, data: Value, private_key: &SigningKey) -> Result<(), Box<dyn std::error::Error>> {
        self.data = data;
        self.updated_at = Utc::now();
        self.hash = hash_data(&self.data);
        self.signature = sign_hash(&self.hash, private_key)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sentinel_crypto::SigningKey;
    use rand::{rngs::OsRng, RngCore};

    use super::*;

    #[test]
    fn test_document_creation() {
        let mut rng = OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);
        let private_key = SigningKey::from_bytes(&key_bytes);
        let data = serde_json::json!({"name": "Test", "value": 42});
        let doc = Document::new(
            "test-id".to_string(),
            crate::META_SENTINEL_VERSION,
            data.clone(),
            &private_key,
        )
        .unwrap();

        assert_eq!(doc.id(), "test-id");
        assert_eq!(doc.version(), crate::META_SENTINEL_VERSION);
        assert_eq!(doc.data(), &data);
        assert!(!doc.hash().is_empty());
        assert!(!doc.signature().is_empty());
        assert_eq!(doc.created_at(), doc.updated_at());
    }

    #[test]
    fn test_document_with_empty_data() {
        let mut rng = OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);
        let private_key = SigningKey::from_bytes(&key_bytes);
        let data = serde_json::json!({});
        let doc = Document::new("empty".to_string(), 1, data.clone(), &private_key).unwrap();

        assert_eq!(doc.id(), "empty");
        assert_eq!(doc.version(), 1);
        assert!(doc.data().as_object().unwrap().is_empty());
    }

    #[test]
    fn test_document_with_complex_data() {
        let mut rng = OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);
        let private_key = SigningKey::from_bytes(&key_bytes);
        let data = serde_json::json!({
            "string": "value",
            "number": 123,
            "boolean": true,
            "array": [1, 2, 3],
            "object": {"nested": "value"}
        });
        let doc = Document::new("complex".to_string(), 1, data.clone(), &private_key).unwrap();

        assert_eq!(doc.data()["string"], "value");
        assert_eq!(doc.data()["number"], 123);
        assert_eq!(doc.data()["boolean"], true);
        assert_eq!(doc.data()["array"], serde_json::json!([1, 2, 3]));
        assert_eq!(doc.data()["object"]["nested"], "value");
    }

    #[test]
    fn test_document_with_valid_filename_safe_ids() {
        let mut rng = OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);
        let private_key = SigningKey::from_bytes(&key_bytes);
        // Test various valid filename-safe document IDs
        let valid_ids = vec![
            "user-123",
            "user_456",
            "user123",
            "123",
            "a",
            "user-123_test",
            "CamelCaseID",
        ];

        for id in valid_ids {
            let data = serde_json::json!({"data": "test"});
            let doc = Document::new(id.to_owned(), 1, data.clone(), &private_key).unwrap();

            assert_eq!(doc.id(), id);
            assert_eq!(doc.data(), &data);
        }
    }

    #[test]
    fn test_set_data_updates_hash_and_signature() {
        let mut rng = OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);
        let private_key = SigningKey::from_bytes(&key_bytes);
        let initial_data = serde_json::json!({"initial": "data"});
        let mut doc = Document::new("test".to_string(), 1, initial_data, &private_key).unwrap();
        let initial_hash = doc.hash().to_string();
        let initial_signature = doc.signature().to_string();
        let initial_updated_at = doc.updated_at();

        let new_data = serde_json::json!({"new": "data"});
        doc.set_data(new_data.clone(), &private_key).unwrap();

        assert_eq!(doc.data(), &new_data);
        assert_ne!(doc.hash(), initial_hash);
        assert_ne!(doc.signature(), initial_signature);
        assert!(doc.updated_at() > initial_updated_at);
    }
}
