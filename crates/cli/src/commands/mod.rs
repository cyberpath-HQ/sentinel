use clap::{Args, Parser, Subcommand};

/// Command handlers for the Sentinel CLI.
///
/// This module contains submodules for each CLI command, each implementing
/// the logic for a specific operation on the Sentinel DBMS.
/// Collection command module.
mod collection;
/// Store command module.
mod store;
/// WAL command module.
mod wal;

/// Parse hash algorithm string to enum
fn parse_hash_algorithm(s: &str) -> Result<sentinel_dbms::HashAlgorithmChoice, String> {
    match s {
        "blake3" => Ok(sentinel_dbms::HashAlgorithmChoice::Blake3),
        _ => Err(format!("Invalid hash algorithm: {}", s)),
    }
}

/// Parse signature algorithm string to enum
fn parse_signature_algorithm(s: &str) -> Result<sentinel_dbms::SignatureAlgorithmChoice, String> {
    match s {
        "ed25519" => Ok(sentinel_dbms::SignatureAlgorithmChoice::Ed25519),
        _ => Err(format!("Invalid signature algorithm: {}", s)),
    }
}

/// Parse encryption algorithm string to enum
fn parse_encryption_algorithm(s: &str) -> Result<sentinel_dbms::EncryptionAlgorithmChoice, String> {
    match s {
        "xchacha20poly1305" => Ok(sentinel_dbms::EncryptionAlgorithmChoice::XChaCha20Poly1305),
        "aes256gcmsiv" => Ok(sentinel_dbms::EncryptionAlgorithmChoice::Aes256GcmSiv),
        "ascon128" => Ok(sentinel_dbms::EncryptionAlgorithmChoice::Ascon128),
        _ => Err(format!("Invalid encryption algorithm: {}", s)),
    }
}

/// Parse key derivation algorithm string to enum
fn parse_key_derivation_algorithm(s: &str) -> Result<sentinel_dbms::KeyDerivationAlgorithmChoice, String> {
    match s {
        "argon2id" => Ok(sentinel_dbms::KeyDerivationAlgorithmChoice::Argon2id),
        "pbkdf2" => Ok(sentinel_dbms::KeyDerivationAlgorithmChoice::Pbkdf2),
        _ => Err(format!("Invalid key derivation algorithm: {}", s)),
    }
}

/// Global WAL configuration arguments shared across all commands.
#[derive(Args, Clone, Default)]
pub struct WalArgs {
    /// Maximum WAL file size in bytes for collections (default: 10MB)
    #[arg(long, global = true)]
    pub wal_max_file_size: Option<u64>,

    /// WAL file format for collections: binary or json_lines (default: binary)
    #[arg(long, global = true)]
    pub wal_format: Option<sentinel_dbms::WalFormat>,

    /// WAL compression algorithm for collections: zstd, lz4, brotli, deflate, gzip (default: zstd)
    #[arg(long, global = true)]
    pub wal_compression: Option<sentinel_dbms::CompressionAlgorithm>,

    /// Maximum number of records per WAL file for collections (default: 1000)
    #[arg(long, global = true)]
    pub wal_max_records: Option<usize>,

    /// WAL write mode for collections: disabled, warn, strict (default: strict)
    #[arg(long, global = true)]
    pub wal_write_mode: Option<sentinel_dbms::WalFailureMode>,

    /// WAL verification mode for collections: disabled, warn, strict (default: warn)
    #[arg(long, global = true)]
    pub wal_verify_mode: Option<sentinel_dbms::WalFailureMode>,

    /// Enable automatic document verification against WAL for collections (default: false)
    #[arg(long, global = true)]
    pub wal_auto_verify: Option<bool>,

    /// Enable WAL-based recovery features for collections (default: true)
    #[arg(long, global = true)]
    pub wal_enable_recovery: Option<bool>,

    /// Persist WAL configuration overrides to disk for existing collections (default: false)
    #[arg(long, global = true)]
    pub wal_persist_overrides: bool,
}

/// The CLI for the Sentinel document DBMS.
///
/// This CLI provides commands to interact with Sentinel stores, collections, and documents.
/// It supports various operations like initializing stores, managing collections, and CRUD
/// operations on documents.
#[derive(Parser)]
#[command(name = "sentinel")]
pub struct Cli {
    #[command(subcommand)]
    /// The subcommand to execute.
    pub command: Commands,

    /// Output logs in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    /// Increase verbosity (can be used multiple times: -v for debug, -vv for trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Hash algorithm to use for cryptographic operations & data integrity.
    ///
    /// Options:
    /// - blake3 (fast, secure, default)
    #[arg(long, value_name = "ALGORITHM", default_value = "blake3", value_parser = ["blake3"], global = true)]
    pub hash_algorithm: String,

    /// Signature algorithm to use for cryptographic operations & authentication.
    ///
    /// Options:
    /// - ed25519 (secure, performant, default)
    #[arg(long, value_name = "ALGORITHM", default_value = "ed25519", value_parser = ["ed25519"], global = true)]
    pub signature_algorithm: String,

    /// Encryption algorithm to use for cryptographic operations & data protection.
    ///
    /// Options:
    /// - xchacha20poly1305 (strongest security, nonce-misuse resistant, default)
    /// - aes256gcmsiv (strong security, nonce-misuse resistant)
    /// - ascon128 (lightweight, good security for constrained environments)
    #[arg(long, value_name = "ALGORITHM", default_value = "xchacha20poly1305", value_parser = ["xchacha20poly1305", "aes256gcmsiv", "ascon128"], global = true)]
    pub encryption_algorithm: String,

    /// Key derivation algorithm to use for cryptographic operations & passphrase-based key
    /// generation.
    ///
    /// Options:
    /// - argon2id (strong security against attacks, default)
    /// - pbkdf2 (widely supported for constrained environments)
    #[arg(long, value_name = "ALGORITHM", default_value = "argon2id", value_parser = ["argon2id", "pbkdf2"], global = true)]
    pub key_derivation_algorithm: String,

    /// WAL (Write-Ahead Log) configuration options for the store.
    #[command(flatten)]
    pub wal: WalArgs,
}

/// Enumeration of all available CLI commands.
///
/// Each variant represents a different operation that can be performed on the Sentinel DBMS.
#[derive(Subcommand)]
pub enum Commands {
    /// Store management operations.
    ///
    /// Provides commands for initializing stores, generating keys, listing collections,
    /// and deleting collections.
    Store(store::StoreArgs),
    /// Collection management operations.
    ///
    /// Provides commands for creating collections, and performing CRUD operations
    /// on documents within collections.
    Collection(collection::CollectionArgs),
    /// WAL (Write-Ahead Logging) management operations.
    ///
    /// Provides commands for checkpointing, verification, recovery, and configuration
    /// of WAL files for collections and the entire store.
    Wal(wal::WalArgs),
}

/// Execute the specified CLI command.
///
/// This function dispatches to the appropriate command handler based on the
/// provided command variant, delegating the actual work to isolated modules.
/// It also initializes the global crypto configuration based on the provided
/// algorithm flags.
///
/// # Arguments
/// * `cli` - The parsed CLI arguments.
///
/// # Returns
/// Returns `Ok(())` on success, or an `io::Error` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::{run_command, Cli};
///
/// let cli = Cli::parse();
/// run_command(cli).await?;
/// ```
pub async fn run_command(cli: Cli) -> sentinel_dbms::Result<()> {
    // Parse algorithms
    let hash_alg = parse_hash_algorithm(&cli.hash_algorithm).map_err(|e| {
        sentinel_dbms::SentinelError::ConfigError {
            message: e,
        }
    })?;
    let sig_alg = parse_signature_algorithm(&cli.signature_algorithm).map_err(|e| {
        sentinel_dbms::SentinelError::ConfigError {
            message: e,
        }
    })?;
    let enc_alg = parse_encryption_algorithm(&cli.encryption_algorithm).map_err(|e| {
        sentinel_dbms::SentinelError::ConfigError {
            message: e,
        }
    })?;
    let kd_alg = parse_key_derivation_algorithm(&cli.key_derivation_algorithm).map_err(|e| {
        sentinel_dbms::SentinelError::ConfigError {
            message: e,
        }
    })?;

    let config = sentinel_dbms::CryptoConfig {
        hash_algorithm:           hash_alg,
        signature_algorithm:      sig_alg,
        encryption_algorithm:     enc_alg,
        key_derivation_algorithm: kd_alg,
    };

    if let Err(err) = sentinel_dbms::set_global_crypto_config(config.clone()).await {
        // If already set, check if it's the same config
        let current = sentinel_dbms::get_global_crypto_config().await?;
        if current != config {
            return Err(sentinel_dbms::SentinelError::ConfigError {
                message: format!("Crypto config already set with different values: {}", err),
            });
        }
    }

    match cli.command {
        Commands::Store(args) => store::run(args).await,
        Commands::Collection(args) => collection::run(args).await,
        Commands::Wal(args) => wal::run(args).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test CLI command parsing.
    ///
    /// This test verifies that the CLI correctly parses various commands
    /// and their arguments using clap's testing utilities.
    #[test]
    fn test_cli_parsing() {
        // Test store init command
        let cli_parsed = Cli::try_parse_from(["test", "store", "init", "--path", "/tmp/store"]).unwrap();
        match cli_parsed.command {
            Commands::Store(args) => {
                match args.subcommand {
                    store::StoreCommands::Init(init_args) => assert_eq!(init_args.path, "/tmp/store"),
                    _ => panic!("Expected Store Init command"),
                }
            },
            _ => panic!("Expected Store command"),
        }

        // Test collection create command
        let cli_parsed = Cli::try_parse_from([
            "test",
            "collection",
            "--store",
            "/tmp/store",
            "--name",
            "users",
            "create",
        ])
        .unwrap();
        match cli_parsed.command {
            Commands::Collection(args) => {
                assert_eq!(args.store, "/tmp/store");
                assert_eq!(args.name, "users");
                match args.command {
                    collection::CollectionCommands::Create(_) => {},
                    _ => panic!("Expected Create subcommand"),
                }
            },
            _ => panic!("Expected Collection command"),
        }

        // Test collection insert command
        let cli_parsed = Cli::try_parse_from([
            "test",
            "collection",
            "--store",
            "/tmp/store",
            "--name",
            "users",
            "insert",
            "--id",
            "user1",
            "--data",
            "{}",
        ])
        .unwrap();
        match cli_parsed.command {
            Commands::Collection(args) => {
                assert_eq!(args.store, "/tmp/store");
                assert_eq!(args.name, "users");
                match args.command {
                    collection::CollectionCommands::Insert(insert_args) => {
                        assert_eq!(insert_args.id, Some(String::from("user1")));
                        assert_eq!(insert_args.data, Some(String::from("{}")));
                    },
                    _ => panic!("Expected Insert subcommand"),
                }
            },
            _ => panic!("Expected Collection command"),
        }
    }

    /// Test CLI with verbose flag.
    ///
    /// This test checks that the verbose flag is parsed correctly.
    #[test]
    fn test_cli_verbose_parsing() {
        let cli_parsed = Cli::try_parse_from(["test", "-v", "store", "init", "--path", "/tmp/store"]).unwrap();
        assert_eq!(cli_parsed.verbose, 1);

        let cli_parsed = Cli::try_parse_from(["test", "-vv", "store", "init", "--path", "/tmp/store"]).unwrap();
        assert_eq!(cli_parsed.verbose, 2);
    }

    /// Test CLI with JSON flag.
    ///
    /// This test verifies that the JSON output flag is parsed correctly.
    #[test]
    fn test_cli_json_parsing() {
        let cli_parsed = Cli::try_parse_from(["test", "--json", "store", "init", "--path", "/tmp/store"]).unwrap();
        assert!(cli_parsed.json);
    }

    /// Test invalid command.
    ///
    /// This test ensures that invalid commands are rejected.
    #[test]
    fn test_invalid_command() {
        let result = Cli::try_parse_from(["test", "invalid-command"]);
        assert!(result.is_err(), "Invalid command should be rejected");
    }

    /// Test missing required arguments.
    ///
    /// This test checks that commands fail when required arguments are missing.
    #[test]
    fn test_missing_required_args() {
        // Init without path
        let result = Cli::try_parse_from(["test", "init"]);
        assert!(result.is_err(), "Init should require path argument");

        // Collection create without name
        let result = Cli::try_parse_from(["test", "collection", "--store", "/tmp", "create"]);
        assert!(
            result.is_err(),
            "Collection create should require name argument"
        );
    }

    /// Test run_command with Init command.
    ///
    /// This test verifies that run_command correctly dispatches to init::run.
    #[tokio::test]
    async fn test_run_command_init() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        let init_args = super::store::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        let args = super::store::StoreArgs {
            subcommand: super::store::StoreCommands::Init(init_args),
        };
        let cli = Cli {
            command: Commands::Store(args),
            json: false,
            verbose: 0,
            hash_algorithm: String::from("blake3"),
            signature_algorithm: String::from("ed25519"),
            encryption_algorithm: String::from("xchacha20poly1305"),
            key_derivation_algorithm: String::from("argon2id"),
            wal: WalArgs::default(),
        };

        let result = run_command(cli).await;
        assert!(result.is_ok(), "run_command should succeed for valid Init");
    }

    /// Test run_command with CreateCollection command.
    ///
    /// This test verifies that run_command correctly dispatches to create_collection::run.
    #[tokio::test]
    async fn test_run_command_create_collection() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store
        let init_args = super::store::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        let store_args = super::store::StoreArgs {
            subcommand: super::store::StoreCommands::Init(init_args),
        };
        let init_cli = Cli {
            command: Commands::Store(store_args),
            json: false,
            verbose: 0,
            hash_algorithm: String::from("blake3"),
            signature_algorithm: String::from("ed25519"),
            encryption_algorithm: String::from("xchacha20poly1305"),
            key_derivation_algorithm: String::from("argon2id"),
            wal: WalArgs::default(),
        };
        run_command(init_cli).await.unwrap();

        let collection_args = super::collection::CollectionArgs {
            store:      store_path.to_string_lossy().to_string(),
            name:       String::from("test_collection"),
            passphrase: None,
            command:    super::collection::CollectionCommands::Create(super::collection::create::CreateArgs::default()),
        };
        let cli = Cli {
            command: Commands::Collection(collection_args),
            json: false,
            verbose: 0,
            hash_algorithm: String::from("blake3"),
            signature_algorithm: String::from("ed25519"),
            encryption_algorithm: String::from("xchacha20poly1305"),
            key_derivation_algorithm: String::from("argon2id"),
            wal: WalArgs::default(),
        };

        let result = run_command(cli).await;
        assert!(
            result.is_ok(),
            "run_command should succeed for valid Collection Create"
        );
    }

    /// Test run_command with invalid algorithm.
    ///
    /// This test verifies that run_command fails with invalid algorithm names.
    #[tokio::test]
    async fn test_run_command_invalid_algorithm() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        let init_args = super::store::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        let args = super::store::StoreArgs {
            subcommand: super::store::StoreCommands::Init(init_args),
        };
        let cli = Cli {
            command: Commands::Store(args),
            json: false,
            verbose: 0,
            hash_algorithm: String::from("invalid"),
            signature_algorithm: String::from("ed25519"),
            encryption_algorithm: String::from("xchacha20poly1305"),
            key_derivation_algorithm: String::from("argon2id"),
            wal: WalArgs::default(),
        };

        let result = run_command(cli).await;
        assert!(
            result.is_err(),
            "run_command should fail with invalid hash algorithm"
        );
    }

    #[test]
    fn test_parse_hash_algorithm_valid() {
        assert_eq!(
            parse_hash_algorithm("blake3"),
            Ok(sentinel_dbms::HashAlgorithmChoice::Blake3)
        );
    }

    #[test]
    fn test_parse_hash_algorithm_invalid() {
        assert!(parse_hash_algorithm("invalid").is_err());
    }

    #[test]
    fn test_parse_signature_algorithm_valid() {
        assert_eq!(
            parse_signature_algorithm("ed25519"),
            Ok(sentinel_dbms::SignatureAlgorithmChoice::Ed25519)
        );
    }

    #[test]
    fn test_parse_signature_algorithm_invalid() {
        assert!(parse_signature_algorithm("invalid").is_err());
    }

    #[test]
    fn test_parse_encryption_algorithm_valid() {
        assert_eq!(
            parse_encryption_algorithm("xchacha20poly1305"),
            Ok(sentinel_dbms::EncryptionAlgorithmChoice::XChaCha20Poly1305)
        );
        assert_eq!(
            parse_encryption_algorithm("aes256gcmsiv"),
            Ok(sentinel_dbms::EncryptionAlgorithmChoice::Aes256GcmSiv)
        );
        assert_eq!(
            parse_encryption_algorithm("ascon128"),
            Ok(sentinel_dbms::EncryptionAlgorithmChoice::Ascon128)
        );
    }

    #[test]
    fn test_parse_encryption_algorithm_invalid() {
        assert!(parse_encryption_algorithm("invalid").is_err());
    }

    #[test]
    fn test_parse_key_derivation_algorithm_valid() {
        assert_eq!(
            parse_key_derivation_algorithm("argon2id"),
            Ok(sentinel_dbms::KeyDerivationAlgorithmChoice::Argon2id)
        );
        assert_eq!(
            parse_key_derivation_algorithm("pbkdf2"),
            Ok(sentinel_dbms::KeyDerivationAlgorithmChoice::Pbkdf2)
        );
    }

    #[test]
    fn test_parse_key_derivation_algorithm_invalid() {
        assert!(parse_key_derivation_algorithm("invalid").is_err());
    }
}

impl WalArgs {
    /// Convert WalArgs to CollectionWalConfigOverrides for merging with stored config.
    pub fn to_overrides(&self) -> sentinel_dbms::CollectionWalConfigOverrides {
        sentinel_dbms::CollectionWalConfigOverrides {
            write_mode:            self.wal_write_mode,
            verification_mode:     self.wal_verify_mode,
            auto_verify:           self.wal_auto_verify,
            enable_recovery:       self.wal_enable_recovery,
            max_wal_size_bytes:    self.wal_max_file_size.map(Some), /* If provided, set to Some(value), else None
                                                                      * (don't override) */
            compression_algorithm: self.wal_compression.map(Some),
            max_records_per_file:  self.wal_max_records.map(Some),
            format:                self.wal_format,
            persist_overrides:     self.wal_persist_overrides,
        }
    }
}
