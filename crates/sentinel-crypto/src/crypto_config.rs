use std::sync::OnceLock;

// Algorithm configuration enums
/// Hash algorithm options for global configuration
#[derive(Clone, Debug)]
pub enum HashAlgorithmChoice {
    Blake3,
}

/// Signature algorithm options for global configuration
#[derive(Clone, Debug)]
pub enum SignatureAlgorithmChoice {
    Ed25519,
}

/// Encryption algorithm options for global configuration
#[derive(Clone, Debug)]
pub enum EncryptionAlgorithmChoice {
    XChaCha20Poly1305,
    Aes256GcmSiv,
    Ascon128,
}

/// Key derivation algorithm options for global configuration
#[derive(Clone, Debug)]
pub enum KeyDerivationAlgorithmChoice {
    Argon2id,
    Pbkdf2,
}

/// Global cryptographic configuration.
/// This allows runtime selection of algorithms for all default operations.
/// Defaults to the most secure algorithms available.
#[derive(Clone, Debug)]
pub struct CryptoConfig {
    pub hash_algorithm: HashAlgorithmChoice,
    pub signature_algorithm: SignatureAlgorithmChoice,
    pub encryption_algorithm: EncryptionAlgorithmChoice,
    pub key_derivation_algorithm: KeyDerivationAlgorithmChoice,
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            hash_algorithm: HashAlgorithmChoice::Blake3,
            signature_algorithm: SignatureAlgorithmChoice::Ed25519,
            encryption_algorithm: EncryptionAlgorithmChoice::XChaCha20Poly1305,
            key_derivation_algorithm: KeyDerivationAlgorithmChoice::Argon2id,
        }
    }
}

// Global configuration storage
static GLOBAL_CONFIG: OnceLock<CryptoConfig> = OnceLock::new();

/// Sets the global cryptographic configuration.
/// This affects all default cryptographic operations.
/// Must be called before any cryptographic operations for the configuration to take effect.
pub fn set_global_crypto_config(config: CryptoConfig) {
    GLOBAL_CONFIG.set(config).expect("Global config can only be set once");
}

/// Gets the current global cryptographic configuration.
/// Returns the default configuration if none has been set.
pub fn get_global_crypto_config() -> &'static CryptoConfig {
    GLOBAL_CONFIG.get_or_init(CryptoConfig::default)
}