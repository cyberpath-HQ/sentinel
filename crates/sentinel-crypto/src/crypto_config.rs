use tokio::sync::RwLock as TokioRwLock;
use tracing::{debug, trace, warn};

use crate::error::CryptoError;

// Algorithm configuration enums
/// Hash algorithm options for global configuration
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum HashAlgorithmChoice {
    /// Blake3 is chosen for its speed and security
    #[default]
    Blake3,
}

/// Signature algorithm options for global configuration
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum SignatureAlgorithmChoice {
    /// Ed25519 is chosen for its security and performance
    #[default]
    Ed25519,
}

/// Encryption algorithm options for global configuration
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum EncryptionAlgorithmChoice {
    /// XChaCha20Poly1305 is chosen for its security and nonce misuse resistance, strongest option
    #[default]
    XChaCha20Poly1305,
    /// Aes256GcmSiv provides strong security with nonce misuse resistance
    Aes256GcmSiv,
    /// Ascon128 is a lightweight authenticated encryption algorithm with good security properties,
    /// suggested for constrained environments
    Ascon128,
}

/// Key derivation algorithm options for global configuration
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum KeyDerivationAlgorithmChoice {
    /// Argon2id is chosen for its strong security properties against various attacks
    #[default]
    Argon2id,
    /// PBKDF2 is a widely supported key derivation function suitable for constrained environments
    Pbkdf2,
}

/// Global cryptographic configuration.
/// This allows runtime selection of algorithms for all default operations.
/// Defaults to the most secure algorithms available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CryptoConfig {
    pub hash_algorithm:           HashAlgorithmChoice,
    pub signature_algorithm:      SignatureAlgorithmChoice,
    pub encryption_algorithm:     EncryptionAlgorithmChoice,
    pub key_derivation_algorithm: KeyDerivationAlgorithmChoice,
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            hash_algorithm:           HashAlgorithmChoice::Blake3,
            signature_algorithm:      SignatureAlgorithmChoice::Ed25519,
            encryption_algorithm:     EncryptionAlgorithmChoice::XChaCha20Poly1305,
            key_derivation_algorithm: KeyDerivationAlgorithmChoice::Argon2id,
        }
    }
}

// Global configuration storage
static GLOBAL_CONFIG: TokioRwLock<Option<CryptoConfig>> = TokioRwLock::const_new(None);

/// Sets the global cryptographic configuration.
/// This affects all default cryptographic operations.
/// Can be called multiple times, but a warning is emitted if the config is changed.
pub async fn set_global_crypto_config(config: CryptoConfig) -> Result<(), CryptoError> {
    trace!("Setting global crypto config: {:?}", config);
    let mut global = GLOBAL_CONFIG.write().await;
    if global.is_some() {
        warn!("Global crypto config is being changed. This may affect ongoing operations.");
    }
    *global = Some(config);
    debug!("Global crypto config set successfully");
    Ok(())
}

/// Gets the current global cryptographic configuration.
/// Returns the default configuration if none has been set.
pub async fn get_global_crypto_config() -> Result<CryptoConfig, CryptoError> {
    trace!("Retrieving global crypto config");
    // First try to read
    {
        let global = GLOBAL_CONFIG.read().await;
        if let Some(ref config) = *global {
            debug!("Global crypto config retrieved: {:?}", config);
            return Ok(config.clone());
        }
    }
    // If none, initialize with write lock
    let mut global = GLOBAL_CONFIG.write().await;
    if global.is_none() {
        *global = Some(CryptoConfig::default());
    }
    let config = global.as_ref().unwrap();
    debug!("Global crypto config retrieved: {:?}", config);
    Ok(config.clone())
}

/// Resets the global cryptographic configuration for testing purposes.
/// This allows tests to set different configurations.
#[cfg(test)]
pub async fn reset_global_crypto_config_for_tests() {
    let mut global = GLOBAL_CONFIG.write().await;
    *global = None;
}
