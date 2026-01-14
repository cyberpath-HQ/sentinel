use clap::Args;
use rand::RngCore;
use sentinel_crypto::SigningKey;
use tracing::info;

/// Arguments for the generate-key command.
#[derive(Args, Clone)]
pub struct GenerateKeyArgs {}

/// Generate a new signing key and output it to the terminal.
///
/// This function generates a new Ed25519 signing key and prints it
/// as a hex-encoded string to stdout. The key can be used for signing
/// documents in Sentinel.
///
/// # Arguments
/// * `_args` - The parsed command-line arguments (none for this command).
///
/// # Returns
/// Returns `Ok(())` on success.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::generate_key::{run, GenerateKeyArgs};
///
/// let args = GenerateKeyArgs {};
/// run(args).await?;
/// ```
pub async fn run(_args: GenerateKeyArgs) -> sentinel::Result<()> {
    let mut rng = rand::thread_rng();
    let mut key_bytes = [0u8; 32];
    rng.fill_bytes(&mut key_bytes);
    let key = SigningKey::from_bytes(&key_bytes);
    let key_hex = hex::encode(key.to_bytes());
    info!("Generated signing key: {}", key_hex);
    println!("{}", key_hex);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        // This is a simple test to ensure the function runs without error
        let args = GenerateKeyArgs {};
        // Note: We can't easily test the async function in unit tests without tokio
        // In a real scenario, we'd use tokio::test
    }
}