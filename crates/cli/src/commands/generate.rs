use clap::{Args, Subcommand};

/// Arguments for the gen command.
#[derive(Args)]
pub struct GenArgs {
    #[command(subcommand)]
    /// The gen subcommand to execute.
    pub subcommand: GenCommands,
}

/// Enumeration of gen subcommands.
#[derive(Subcommand)]
pub enum GenCommands {
    /// Generate a cryptographic key.
    ///
    /// This command generates a new key of the specified type and outputs it as a hex string.
    Key(KeyArgs),
}

/// Arguments for the gen key command.
#[derive(Args)]
pub struct KeyArgs {
    /// The type of key to generate: signing or encryption.
    #[arg(value_enum)]
    pub key_type: KeyType,
}

/// The type of key to generate.
#[derive(Clone, clap::ValueEnum)]
pub enum KeyType {
    /// Generate a signing key (Ed25519).
    Signing,
    /// Generate an encryption key (256-bit).
    Encryption,
}

/// Run the gen command.
///
/// # Arguments
/// * `args` - The parsed gen command arguments.
///
/// # Returns
/// Returns `Ok(())` on success.
pub async fn run(args: GenArgs) -> sentinel::Result<()> {
    match args.subcommand {
        GenCommands::Key(key_args) => run_key(key_args).await,
    }
}

/// Run the gen key command.
///
/// # Arguments
/// * `args` - The parsed key command arguments.
///
/// # Returns
/// Returns `Ok(())` on success.
pub async fn run_key(args: KeyArgs) -> sentinel::Result<()> {
    match args.key_type {
        KeyType::Signing => {
            let key = sentinel_crypto::SigningKeyManager::generate_key();
            let key_hex = sentinel_crypto::SigningKeyManager::export_key(&key);
            #[allow(clippy::print_stdout, reason = "CLI output")]
            {
                println!("{}", key_hex);
            }
        },
        KeyType::Encryption => {
            let key = sentinel_crypto::EncryptionKeyManager::generate_key();
            let key_hex = sentinel_crypto::EncryptionKeyManager::export_key(&key);
            #[allow(clippy::print_stdout, reason = "CLI output")]
            {
                println!("{}", key_hex);
            }
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_key_signing() {
        let args = KeyArgs {
            key_type: KeyType::Signing,
        };
        // Note: We can't easily test the async function output in unit tests without capturing stdout
        // In a real scenario, we'd use tokio::test or capture stdout
        // For now, just ensure it doesn't panic
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(run_key(args));
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_key_encryption() {
        let args = KeyArgs {
            key_type: KeyType::Encryption,
        };
        // Note: We can't easily test the async function output in unit tests without capturing stdout
        // In a real scenario, we'd use tokio::test or capture stdout
        // For now, just ensure it doesn't panic
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(run_key(args));
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_gen_key() {
        let args = GenArgs {
            subcommand: GenCommands::Key(KeyArgs {
                key_type: KeyType::Signing,
            }),
        };
        let result = tokio::runtime::Runtime::new().unwrap().block_on(run(args));
        assert!(result.is_ok());
    }
}
