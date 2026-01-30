use chrono::{DateTime, Utc};
use sentinel_crypto::{hash_data, sign_hash, SigningKey};
use serde_json::Value;
use tracing::{debug, trace};

use crate::Result;

/// Represents a document in the database.
#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::field_scoped_visibility_modifiers,
    reason = "fields need to be pub(crate) for internal access"
)]
pub struct Document {
    /// The unique identifier of the document.
    pub(crate) id:                 String,
    /// The version of the document, represents the version of the client that created it.
    pub(crate) version:            u32,
    /// The timestamp when the document was created.
    pub(crate) created_at:         DateTime<Utc>,
    /// The timestamp when the document was last updated.
    pub(crate) updated_at:         DateTime<Utc>,
    /// The hash of the document data.
    pub(crate) hash:               String,
    /// The signature of the document data.
    pub(crate) signature:          String,
    /// The JSON data of the document.
    pub(crate) data:               Value,
    /// Indicates whether this document was read during a concurrent write operation.
    /// Set to true when reads occur while an exclusive lock is held for a different path.
    pub(crate) stale_data_warning: bool,
    /// Timestamp when the stale data warning was set.
    /// None if no warning is set.
    pub(crate) warning_timestamp:  Option<DateTime<Utc>>,
}

impl Document {
    /// Creates a new document with the given id, version, and data.
    /// Computes the hash and signature using the provided private key.
    pub async fn new(id: String, data: Value, private_key: &SigningKey) -> Result<Self> {
        trace!("Creating new signed document with id: {}", id);
        let now = Utc::now();
        let hash = hash_data(&data).await?;
        let signature = sign_hash(&hash, private_key).await?;
        debug!("Document {} created with hash: {}", id, hash);
        Ok(Self {
            id,
            version: crate::DOCUMENT_SENTINEL_VERSION,
            created_at: now,
            updated_at: now,
            hash,
            signature,
            data,
            stale_data_warning: false,
            warning_timestamp: None,
        })
    }

    /// Creates a new document with the given id and data.
    /// Computes the hash but not the signature.
    pub async fn new_without_signature(id: String, data: Value) -> Result<Self> {
        trace!("Creating new unsigned document with id: {}", id);
        let now = Utc::now();
        let hash = hash_data(&data).await?;
        debug!("Document {} created without signature, hash: {}", id, hash);
        Ok(Self {
            id,
            version: crate::DOCUMENT_SENTINEL_VERSION,
            created_at: now,
            updated_at: now,
            hash,
            signature: String::new(),
            data,
            stale_data_warning: false,
            warning_timestamp: None,
        })
    }

    /// Returns the document ID.
    pub fn id(&self) -> &str { &self.id }

    /// Returns the document version.
    pub const fn version(&self) -> u32 { self.version }

    /// Returns the creation timestamp.
    pub const fn created_at(&self) -> DateTime<Utc> { self.created_at }

    /// Returns the last update timestamp.
    pub const fn updated_at(&self) -> DateTime<Utc> { self.updated_at }

    /// Returns the hash of the document data.
    pub fn hash(&self) -> &str { &self.hash }

    /// Returns the signature of the document data.
    pub fn signature(&self) -> &str { &self.signature }

    /// Returns a reference to the document data.
    pub const fn data(&self) -> &Value { &self.data }

    /// Returns whether this document has a stale data warning.
    /// True if this document was read during a concurrent write operation.
    pub const fn stale_data_warning(&self) -> bool { self.stale_data_warning }

    /// Returns the timestamp when the stale data warning was set.
    /// None if no warning is set.
    pub const fn warning_timestamp(&self) -> Option<DateTime<Utc>> { self.warning_timestamp }

    /// Sets the stale data warning state.
    /// This should only be called internally by the locking system.
    pub(crate) fn set_stale_data_warning(&mut self, warning: bool, timestamp: Option<DateTime<Utc>>) {
        self.stale_data_warning = warning;
        self.warning_timestamp = timestamp;
    }

    /// Verifies the document's signature against the provided public key.
    ///
    /// Returns `Ok(())` if the signature is valid, or an error if invalid.
    pub async fn verify_signature(&self, public_key: &sentinel_crypto::VerifyingKey) -> Result<()> {
        use sentinel_crypto::verify_signature;

        if self.signature.is_empty() {
            return Err(crate::SentinelError::SignatureVerificationFailed {
                id:     self.id.clone(),
                reason: "Document has no signature".to_string(),
            });
        }

        let is_valid = verify_signature(&self.hash, &self.signature, public_key)
            .await
            .map_err(|e| {
                crate::SentinelError::SignatureVerificationFailed {
                    id:     self.id.clone(),
                    reason: format!("Signature verification failed: {}", e),
                }
            })?;

        if !is_valid {
            return Err(crate::SentinelError::SignatureVerificationFailed {
                id:     self.id.clone(),
                reason: "Signature is invalid".to_string(),
            });
        }

        Ok(())
    }

    /// Verifies the document's hash against the current data.
    ///
    /// Returns `Ok(())` if the hash matches, or an error if it doesn't.
    pub async fn verify_hash(&self) -> Result<()> {
        let computed_hash = sentinel_crypto::hash_data(&self.data).await.map_err(|e| {
            crate::SentinelError::HashVerificationFailed {
                id:     self.id.clone(),
                reason: format!("Hash computation failed: {}", e),
            }
        })?;

        if computed_hash != self.hash {
            return Err(crate::SentinelError::HashVerificationFailed {
                id:     self.id.clone(),
                reason: format!(
                    "Hash mismatch: expected {}, got {}",
                    self.hash, computed_hash
                ),
            });
        }

        Ok(())
    }

    /// Sets the document data, updates the hash and signature, and refreshes the updated_at
    /// timestamp.
    pub async fn set_data(&mut self, data: Value, private_key: &SigningKey) -> Result<()> {
        trace!("Updating data for document: {}", self.id);
        self.data = data;
        self.updated_at = Utc::now();
        self.hash = hash_data(&self.data).await?;
        self.signature = sign_hash(&self.hash, private_key).await?;
        debug!("Document {} data updated, new hash: {}", self.id, self.hash);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::OsRng, RngCore};
    use sentinel_crypto::SigningKey;

    use super::*;

    #[tokio::test]
    async fn test_document_creation() {
        let mut rng = OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);
        let private_key = SigningKey::from_bytes(&key_bytes);
        let data = serde_json::json!({"name": "Test", "value": 42});
        let doc = Document::new("test-id".to_string(), data.clone(), &private_key)
            .await
            .unwrap();

        assert_eq!(doc.id(), "test-id");
        assert_eq!(doc.version(), crate::DOCUMENT_SENTINEL_VERSION);
        assert_eq!(doc.data(), &data);
        assert!(!doc.hash().is_empty());
        assert!(!doc.signature().is_empty());
        assert_eq!(doc.created_at(), doc.updated_at());
        assert_eq!(doc.stale_data_warning(), false);
        assert_eq!(doc.warning_timestamp(), None);
    }

    #[tokio::test]
    async fn test_document_with_empty_data() {
        let mut rng = OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);
        let private_key = SigningKey::from_bytes(&key_bytes);
        let data = serde_json::json!({});
        let doc = Document::new("empty".to_string(), data.clone(), &private_key)
            .await
            .unwrap();

        assert_eq!(doc.id(), "empty");
        assert_eq!(doc.version(), crate::DOCUMENT_SENTINEL_VERSION);
        assert!(doc.data().as_object().unwrap().is_empty());
        assert_eq!(doc.stale_data_warning(), false);
        assert_eq!(doc.warning_timestamp(), None);
    }

    #[tokio::test]
    async fn test_document_with_complex_data() {
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
        let doc = Document::new("complex".to_string(), data.clone(), &private_key)
            .await
            .unwrap();

        assert_eq!(doc.data()["string"], "value");
        assert_eq!(doc.data()["number"], 123);
        assert_eq!(doc.data()["boolean"], true);
        assert_eq!(doc.data()["array"], serde_json::json!([1, 2, 3]));
        assert_eq!(doc.data()["object"]["nested"], "value");
    }

    #[tokio::test]
    async fn test_document_with_valid_filename_safe_ids() {
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
            let doc = Document::new(id.to_owned(), data.clone(), &private_key)
                .await
                .unwrap();

            assert_eq!(doc.id(), id);
            assert_eq!(doc.data(), &data);
        }
    }

    #[tokio::test]
    async fn test_set_data_updates_hash_and_signature() {
        let mut rng = OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);
        let private_key = SigningKey::from_bytes(&key_bytes);
        let initial_data = serde_json::json!({"initial": "data"});
        let mut doc = Document::new("test".to_string(), initial_data, &private_key)
            .await
            .unwrap();
        let initial_hash = doc.hash().to_string();
        let initial_signature = doc.signature().to_string();
        let initial_updated_at = doc.updated_at();

        let new_data = serde_json::json!({"new": "data"});
        doc.set_data(new_data.clone(), &private_key).await.unwrap();

        assert_eq!(doc.data(), &new_data);
        assert_ne!(doc.hash(), initial_hash);
        assert_ne!(doc.signature(), initial_signature);
        assert!(doc.updated_at() > initial_updated_at);
    }

    #[tokio::test]
    async fn test_document_getters() {
        let mut rng = OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);
        let private_key = SigningKey::from_bytes(&key_bytes);
        let data = serde_json::json!({"test": "data"});
        let mut doc = Document::new("test_id".to_string(), data.clone(), &private_key)
            .await
            .unwrap();

        // Test all getter methods
        assert_eq!(doc.id(), "test_id");
        assert_eq!(doc.version(), crate::DOCUMENT_SENTINEL_VERSION);
        assert!(doc.created_at() <= Utc::now());
        assert!(doc.updated_at() <= Utc::now());
        assert!(!doc.hash().is_empty());
        assert!(!doc.signature().is_empty());
        assert_eq!(doc.data(), &data);
        assert_eq!(doc.stale_data_warning(), false);
        assert_eq!(doc.warning_timestamp(), None);

        // Test set_data to cover the closure inside it
        let new_data = serde_json::json!({"updated": "data"});
        doc.set_data(new_data.clone(), &private_key).await.unwrap();
        assert_eq!(doc.data(), &new_data);
        // Warning fields should remain false after update
        assert_eq!(doc.stale_data_warning(), false);
        assert_eq!(doc.warning_timestamp(), None);
    }

    #[tokio::test]
    async fn test_stale_data_warning_defaults() {
        let mut rng = OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);
        let private_key = SigningKey::from_bytes(&key_bytes);
        let data = serde_json::json!({"test": "data"});
        let doc = Document::new("test-id".to_string(), data, &private_key)
            .await
            .unwrap();

        // Warning fields should be false and None by default
        assert_eq!(doc.stale_data_warning(), false);
        assert_eq!(doc.warning_timestamp(), None);
    }

    #[tokio::test]
    async fn test_set_data_preserves_warning_state() {
        let mut rng = OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);
        let private_key = SigningKey::from_bytes(&key_bytes);

        let mut doc = Document::new(
            "test".to_string(),
            serde_json::json!({"initial": "data"}),
            &private_key,
        )
        .await
        .unwrap();

        // Initially no warning
        assert_eq!(doc.stale_data_warning(), false);

        // Update data - warning should remain false
        doc.set_data(serde_json::json!({"new": "data"}), &private_key)
            .await
            .unwrap();
        assert_eq!(doc.stale_data_warning(), false);
        assert_eq!(doc.warning_timestamp(), None);
    }
}
