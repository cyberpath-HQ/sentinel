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
