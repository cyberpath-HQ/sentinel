use tracing::{error, trace, warn};

use crate::{Result, SentinelError};
use super::collection::Collection;

impl Collection {
    /// Verifies document hash according to the specified verification options.
    ///
    /// # Arguments
    ///
    /// * `doc` - The document to verify
    /// * `options` - The verification options
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if verification passes or is handled according to the mode,
    /// or `Err(SentinelError::HashVerificationFailed)` if verification fails in Strict mode.
    pub async fn verify_hash(&self, doc: &crate::Document, options: crate::VerificationOptions) -> Result<()> {
        if options.hash_verification_mode == crate::VerificationMode::Silent {
            return Ok(());
        }

        trace!("Verifying hash for document: {}", doc.id());
        let computed_hash = sentinel_crypto::hash_data(doc.data()).await?;

        if computed_hash != doc.hash() {
            let reason = format!(
                "Expected hash: {}, Computed hash: {}",
                doc.hash(),
                computed_hash
            );

            match options.hash_verification_mode {
                crate::VerificationMode::Strict => {
                    error!("Document {} hash verification failed: {}", doc.id(), reason);
                    return Err(SentinelError::HashVerificationFailed {
                        id: doc.id().to_owned(),
                        reason,
                    });
                },
                crate::VerificationMode::Warn => {
                    warn!("Document {} hash verification failed: {}", doc.id(), reason);
                },
                crate::VerificationMode::Silent => {},
            }
        }
        else {
            trace!("Document {} hash verified successfully", doc.id());
        }

        Ok(())
    }

    /// Verifies document signature according to the specified verification options.
    ///
    /// # Arguments
    ///
    /// * `doc` - The document to verify
    /// * `options` - The verification options containing modes for different scenarios
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if verification passes or is handled according to the mode,
    /// or `Err(SentinelError::SignatureVerificationFailed)` if verification fails in Strict mode.
    pub async fn verify_signature(&self, doc: &crate::Document, options: crate::VerificationOptions) -> Result<()> {
        if options.signature_verification_mode == crate::VerificationMode::Silent &&
            options.empty_signature_mode == crate::VerificationMode::Silent
        {
            return Ok(());
        }

        trace!("Verifying signature for document: {}", doc.id());

        if doc.signature().is_empty() {
            let reason = "Document has no signature".to_owned();

            match options.empty_signature_mode {
                crate::VerificationMode::Strict => {
                    error!("Document {} has no signature: {}", doc.id(), reason);
                    return Err(SentinelError::SignatureVerificationFailed {
                        id: doc.id().to_owned(),
                        reason,
                    });
                },
                crate::VerificationMode::Warn => {
                    warn!("Document {} has no signature: {}", doc.id(), reason);
                },
                crate::VerificationMode::Silent => {},
            }
            return Ok(());
        }

        if !options.verify_signature {
            trace!("Signature verification disabled for document: {}", doc.id());
            return Ok(());
        }

        if let Some(ref signing_key) = self.signing_key {
            let public_key = signing_key.verifying_key();
            let is_valid = sentinel_crypto::verify_signature(doc.hash(), doc.signature(), &public_key).await?;

            if !is_valid {
                let reason = "Signature verification using public key failed".to_owned();

                match options.signature_verification_mode {
                    crate::VerificationMode::Strict => {
                        error!(
                            "Document {} signature verification failed: {}",
                            doc.id(),
                            reason
                        );
                        return Err(SentinelError::SignatureVerificationFailed {
                            id: doc.id().to_owned(),
                            reason,
                        });
                    },
                    crate::VerificationMode::Warn => {
                        warn!(
                            "Document {} signature verification failed: {}",
                            doc.id(),
                            reason
                        );
                    },
                    crate::VerificationMode::Silent => {},
                }
            }
            else {
                trace!("Document {} signature verified successfully", doc.id());
            }
        }
        else {
            trace!("No signing key available for verification, skipping signature check");
        }

        Ok(())
    }

    /// Verifies both hash and signature of a document according to the specified options.
    ///
    /// # Arguments
    ///
    /// * `doc` - The document to verify
    /// * `options` - The verification options
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if verifications pass or are handled according to the modes,
    /// or an error if verification fails in Strict mode.
    pub async fn verify_document(&self, doc: &crate::Document, options: &crate::VerificationOptions) -> Result<()> {
        if options.verify_hash {
            self.verify_hash(doc, *options).await?;
        }

        if options.verify_signature {
            self.verify_signature(doc, *options).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VerificationMode, VerificationOptions, Document, Store};
    use serde_json::json;

    async fn setup_collection_with_signing_key() -> (crate::Collection, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            Some("test_passphrase"),
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("test", None).await.unwrap();
        (collection, temp_dir)
    }

    async fn setup_collection() -> (crate::Collection, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("test", None).await.unwrap();
        (collection, temp_dir)
    }

    #[tokio::test]
    async fn test_verify_hash_silent_mode() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;
        collection.insert("doc1", json!({"name": "test"})).await.unwrap();
        let doc = collection.get("doc1").await.unwrap().unwrap();

        let options = VerificationOptions {
            hash_verification_mode: VerificationMode::Silent,
            ..Default::default()
        };

        let result = collection.verify_hash(&doc, options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_hash_warn_mode() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;
        collection.insert("doc1", json!({"name": "test"})).await.unwrap();
        let doc = collection.get("doc1").await.unwrap().unwrap();

        let options = VerificationOptions {
            hash_verification_mode: VerificationMode::Warn,
            ..Default::default()
        };

        let result = collection.verify_hash(&doc, options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_hash_strict_mode_valid() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;
        collection.insert("doc1", json!({"name": "test"})).await.unwrap();
        let doc = collection.get("doc1").await.unwrap().unwrap();

        let options = VerificationOptions {
            hash_verification_mode: VerificationMode::Strict,
            ..Default::default()
        };

        let result = collection.verify_hash(&doc, options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_hash_strict_mode_corrupted() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;
        collection.insert("doc1", json!({"name": "test"})).await.unwrap();
        let mut doc = collection.get("doc1").await.unwrap().unwrap();

        // Corrupt the hash field
        doc = Document {
            id: doc.id().to_string(),
            version: doc.version(),
            created_at: doc.created_at(),
            updated_at: doc.updated_at(),
            hash: "corrupted_hash".to_string(),
            signature: doc.signature().to_string(),
            data: doc.data().clone(),
        };

        let options = VerificationOptions {
            hash_verification_mode: VerificationMode::Strict,
            ..Default::default()
        };

        let result = collection.verify_hash(&doc, options).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_signature_silent_mode() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;
        collection.insert("doc1", json!({"name": "test"})).await.unwrap();
        let doc = collection.get("doc1").await.unwrap().unwrap();

        let options = VerificationOptions {
            signature_verification_mode: VerificationMode::Silent,
            empty_signature_mode: VerificationMode::Silent,
            ..Default::default()
        };

        let result = collection.verify_signature(&doc, options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_signature_empty_signature_strict() {
        let (collection, _temp_dir) = setup_collection().await;
        // Insert without signature
        collection.insert("doc1", json!({"name": "test"})).await.unwrap();
        let doc = collection.get("doc1").await.unwrap().unwrap();

        let options = VerificationOptions {
            empty_signature_mode: VerificationMode::Strict,
            ..Default::default()
        };

        let result = collection.verify_signature(&doc, options).await;
        assert!(result.is_err());
        if let Err(SentinelError::SignatureVerificationFailed { reason, .. }) = result {
            assert!(reason.contains("no signature"));
        }
    }

    #[tokio::test]
    async fn test_verify_signature_empty_signature_warn() {
        let (collection, _temp_dir) = setup_collection().await;
        collection.insert("doc1", json!({"name": "test"})).await.unwrap();
        let doc = collection.get("doc1").await.unwrap().unwrap();

        let options = VerificationOptions {
            empty_signature_mode: VerificationMode::Warn,
            ..Default::default()
        };

        let result = collection.verify_signature(&doc, options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_signature_disabled() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;
        collection.insert("doc1", json!({"name": "test"})).await.unwrap();
        let doc = collection.get("doc1").await.unwrap().unwrap();

        let options = VerificationOptions {
            verify_signature: false,
            ..Default::default()
        };

        let result = collection.verify_signature(&doc, options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_signature_no_signing_key() {
        let (collection, _temp_dir) = setup_collection().await;
        collection.insert("doc1", json!({"name": "test"})).await.unwrap();
        let doc = collection.get("doc1").await.unwrap().unwrap();

        let options = VerificationOptions {
            signature_verification_mode: VerificationMode::Strict,
            empty_signature_mode: VerificationMode::Silent,
            verify_signature: true,
            ..Default::default()
        };

        // Should skip verification if collection has no signing key
        let result = collection.verify_signature(&doc, options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_document_both_enabled() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;
        collection.insert("doc1", json!({"name": "test"})).await.unwrap();
        let doc = collection.get("doc1").await.unwrap().unwrap();

        let options = VerificationOptions {
            verify_hash: true,
            verify_signature: false,
            empty_signature_mode: VerificationMode::Silent,
            hash_verification_mode: VerificationMode::Strict,
            ..Default::default()
        };

        let result = collection.verify_document(&doc, &options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_document_neither_enabled() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;
        collection.insert("doc1", json!({"name": "test"})).await.unwrap();
        let doc = collection.get("doc1").await.unwrap().unwrap();

        let options = VerificationOptions {
            verify_hash: false,
            verify_signature: false,
            ..Default::default()
        };

        let result = collection.verify_document(&doc, &options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_document_hash_only() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;
        collection.insert("doc1", json!({"test": "data"})).await.unwrap();
        let doc = collection.get("doc1").await.unwrap().unwrap();

        let options = VerificationOptions {
            verify_hash: true,
            verify_signature: false,
            hash_verification_mode: VerificationMode::Strict,
            ..Default::default()
        };

        let result = collection.verify_document(&doc, &options).await;
        assert!(result.is_ok());
    }
}
