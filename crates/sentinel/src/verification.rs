use serde::{Deserialize, Serialize};

/// Verification mode for signature and hash checks.
///
/// Defines how integrity verification failures are handled when reading documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VerificationMode {
    /// Fail with an error when verification fails (default for production).
    /// This is the strictest mode and is recommended for production use.
    #[default]
    Strict,
    /// Emit a warning when verification fails but continue processing.
    /// Useful for auditing or migration scenarios where you want to detect issues
    /// without blocking operations.
    Warn,
    /// Silently ignore verification failures.
    /// Useful for performance-critical scenarios or when documents are known to be unsigned.
    Silent,
}

impl VerificationMode {
    /// Parse verification mode from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "strict" => Some(Self::Strict),
            "warn" => Some(Self::Warn),
            "silent" => Some(Self::Silent),
            _ => None,
        }
    }

    /// Convert verification mode to string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Strict => "strict",
            Self::Warn => "warn",
            Self::Silent => "silent",
        }
    }
}

/// Options for controlling verification behavior when reading documents.
///
/// These options allow fine-grained control over integrity verification at the method level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerificationOptions {
    /// Whether to verify document signatures.
    /// Defaults to true for security.
    pub verify_signature:            bool,
    /// Whether to verify document hashes.
    /// Defaults to true for integrity.
    pub verify_hash:                 bool,
    /// How to handle signature verification failures (invalid signatures).
    /// Defaults to Strict.
    pub signature_verification_mode: VerificationMode,
    /// How to handle empty signature documents.
    /// Defaults to Warn (documents without signatures are common in mixed collections).
    pub empty_signature_mode:        VerificationMode,
    /// How to handle hash verification failures.
    /// Defaults to Strict.
    pub hash_verification_mode:      VerificationMode,
}

impl Default for VerificationOptions {
    fn default() -> Self {
        Self {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: VerificationMode::Strict,
            empty_signature_mode:        VerificationMode::Warn,
            hash_verification_mode:      VerificationMode::Strict,
        }
    }
}

impl VerificationOptions {
    /// Create new verification options with all verifications enabled and strict mode.
    pub fn strict() -> Self {
        Self {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: VerificationMode::Strict,
            empty_signature_mode:        VerificationMode::Strict,
            hash_verification_mode:      VerificationMode::Strict,
        }
    }

    /// Create new verification options with all verifications disabled.
    pub fn disabled() -> Self {
        Self {
            verify_signature:            false,
            verify_hash:                 false,
            signature_verification_mode: VerificationMode::Silent,
            empty_signature_mode:        VerificationMode::Silent,
            hash_verification_mode:      VerificationMode::Silent,
        }
    }

    /// Create new verification options with warnings instead of errors.
    pub fn warn() -> Self {
        Self {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: VerificationMode::Warn,
            empty_signature_mode:        VerificationMode::Warn,
            hash_verification_mode:      VerificationMode::Warn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_mode_from_str() {
        assert_eq!(
            VerificationMode::from_str("strict"),
            Some(VerificationMode::Strict)
        );
        assert_eq!(
            VerificationMode::from_str("warn"),
            Some(VerificationMode::Warn)
        );
        assert_eq!(
            VerificationMode::from_str("silent"),
            Some(VerificationMode::Silent)
        );
        assert_eq!(
            VerificationMode::from_str("STRICT"),
            Some(VerificationMode::Strict)
        );
        assert_eq!(VerificationMode::from_str("invalid"), None);
    }

    #[test]
    fn test_verification_mode_as_str() {
        assert_eq!(VerificationMode::Strict.as_str(), "strict");
        assert_eq!(VerificationMode::Warn.as_str(), "warn");
        assert_eq!(VerificationMode::Silent.as_str(), "silent");
    }

    #[test]
    fn test_verification_options_default() {
        let opts = VerificationOptions::default();
        assert!(opts.verify_signature);
        assert!(opts.verify_hash);
        assert_eq!(opts.signature_verification_mode, VerificationMode::Strict);
        assert_eq!(opts.empty_signature_mode, VerificationMode::Warn);
        assert_eq!(opts.hash_verification_mode, VerificationMode::Strict);
    }

    #[test]
    fn test_verification_options_strict() {
        let opts = VerificationOptions::strict();
        assert!(opts.verify_signature);
        assert!(opts.verify_hash);
        assert_eq!(opts.signature_verification_mode, VerificationMode::Strict);
        assert_eq!(opts.empty_signature_mode, VerificationMode::Strict);
        assert_eq!(opts.hash_verification_mode, VerificationMode::Strict);
    }

    #[test]
    fn test_verification_options_disabled() {
        let opts = VerificationOptions::disabled();
        assert!(!opts.verify_signature);
        assert!(!opts.verify_hash);
        assert_eq!(opts.signature_verification_mode, VerificationMode::Silent);
        assert_eq!(opts.empty_signature_mode, VerificationMode::Silent);
        assert_eq!(opts.hash_verification_mode, VerificationMode::Silent);
    }

    #[test]
    fn test_verification_options_warn() {
        let opts = VerificationOptions::warn();
        assert!(opts.verify_signature);
        assert!(opts.verify_hash);
        assert_eq!(opts.signature_verification_mode, VerificationMode::Warn);
        assert_eq!(opts.empty_signature_mode, VerificationMode::Warn);
        assert_eq!(opts.hash_verification_mode, VerificationMode::Warn);
    }
}
