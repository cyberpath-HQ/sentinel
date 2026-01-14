use clap::{Parser, Subcommand};

/// Command handlers for the Sentinel CLI.
///
/// This module contains submodules for each CLI command, each implementing
/// the logic for a specific operation on the Sentinel DBMS.
/// Create collection command module.
mod create_collection;
/// Delete command module.
mod delete;
/// Generate command module.
mod generate;
/// Get command module.
mod get;
/// Init command module.
mod init;
/// Insert command module.
mod insert;
/// Update command module.
mod update;

/// Parse hash algorithm string to enum
fn parse_hash_algorithm(s: &str) -> Result<sentinel_crypto::HashAlgorithmChoice, String> {
    match s {
        "blake3" => Ok(sentinel_crypto::HashAlgorithmChoice::Blake3),
        _ => Err(format!("Invalid hash algorithm: {}", s)),
    }
}

/// Parse signature algorithm string to enum
fn parse_signature_algorithm(s: &str) -> Result<sentinel_crypto::SignatureAlgorithmChoice, String> {
    match s {
        "ed25519" => Ok(sentinel_crypto::SignatureAlgorithmChoice::Ed25519),
        _ => Err(format!("Invalid signature algorithm: {}", s)),
    }
}

/// Parse encryption algorithm string to enum
fn parse_encryption_algorithm(s: &str) -> Result<sentinel_crypto::EncryptionAlgorithmChoice, String> {
    match s {
        "xchacha20poly1305" => Ok(sentinel_crypto::EncryptionAlgorithmChoice::XChaCha20Poly1305),
        "aes256gcmsiv" => Ok(sentinel_crypto::EncryptionAlgorithmChoice::Aes256GcmSiv),
        "ascon128" => Ok(sentinel_crypto::EncryptionAlgorithmChoice::Ascon128),
        _ => Err(format!("Invalid encryption algorithm: {}", s)),
    }
}

/// Parse key derivation algorithm string to enum
fn parse_key_derivation_algorithm(s: &str) -> Result<sentinel_crypto::KeyDerivationAlgorithmChoice, String> {
    match s {
        "argon2id" => Ok(sentinel_crypto::KeyDerivationAlgorithmChoice::Argon2id),
        "pbkdf2" => Ok(sentinel_crypto::KeyDerivationAlgorithmChoice::Pbkdf2),
        _ => Err(format!("Invalid key derivation algorithm: {}", s)),
    }
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
}

/// Enumeration of all available CLI commands.
///
/// Each variant represents a different operation that can be performed on the Sentinel DBMS.
#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new store at the specified path.
    ///
    /// This command creates the necessary directory structure and metadata for a new Sentinel
    /// store.
    Init(init::InitArgs),
    /// Generate cryptographic keys and other artifacts.
    ///
    /// This command provides subcommands for generating keys and other cryptographic materials.
    #[command(visible_alias = "gen")]
    Generate(generate::GenArgs),
    /// Create a new collection within an existing store.
    ///
    /// Collections are logical groupings of documents within a store.
    CreateCollection(create_collection::CreateCollectionArgs),
    /// Insert a new document into a collection.
    ///
    /// The document data must be valid JSON.
    Insert(insert::InsertArgs),
    /// Retrieve a document from a collection.
    ///
    /// If the document exists, its JSON data is printed to stdout.
    Get(get::GetArgs),
    /// Update an existing document in a collection.
    ///
    /// The entire document is replaced with the new data.
    Update(update::UpdateArgs),
    /// Delete a document from a collection.
    Delete(delete::DeleteArgs),
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
pub async fn run_command(cli: Cli) -> sentinel::Result<()> {
    // Parse algorithms
    let hash_alg = parse_hash_algorithm(&cli.hash_algorithm).map_err(|e| {
        sentinel::SentinelError::ConfigError {
            message: e,
        }
    })?;
    let sig_alg = parse_signature_algorithm(&cli.signature_algorithm).map_err(|e| {
        sentinel::SentinelError::ConfigError {
            message: e,
        }
    })?;
    let enc_alg = parse_encryption_algorithm(&cli.encryption_algorithm).map_err(|e| {
        sentinel::SentinelError::ConfigError {
            message: e,
        }
    })?;
    let kd_alg = parse_key_derivation_algorithm(&cli.key_derivation_algorithm).map_err(|e| {
        sentinel::SentinelError::ConfigError {
            message: e,
        }
    })?;

    let config = sentinel_crypto::CryptoConfig {
        hash_algorithm:           hash_alg,
        signature_algorithm:      sig_alg,
        encryption_algorithm:     enc_alg,
        key_derivation_algorithm: kd_alg,
    };

    sentinel_crypto::set_global_crypto_config(config.clone())
        .map_err(|err| {
            sentinel::SentinelError::ConfigError {
                message: err.to_string(),
            }
        })
        .or_else(|_| {
            // If already set, check if it's the same config
            let current = sentinel_crypto::get_global_crypto_config();
            if *current == config {
                Ok(())
            }
            else {
                Err(sentinel::SentinelError::ConfigError {
                    message: "Crypto config already set with different values".to_owned(),
                })
            }
        })?;

    match cli.command {
        Commands::Init(args) => init::run(args).await,
        Commands::Generate(args) => generate::run(args).await,
        Commands::CreateCollection(args) => create_collection::run(args).await,
        Commands::Insert(args) => insert::run(args).await,
        Commands::Get(args) => get::run(args).await,
        Commands::Update(args) => update::run(args).await,
        Commands::Delete(args) => delete::run(args).await,
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
        // Test init command
        let cli_parsed = Cli::try_parse_from(["test", "init", "--path", "/tmp/store"]).unwrap();
        match cli_parsed.command {
            Commands::Init(args) => assert_eq!(args.path, "/tmp/store"),
            _ => panic!("Expected Init command"),
        }

        // Test create-collection command
        let cli_parsed = Cli::try_parse_from([
            "test",
            "create-collection",
            "--store-path",
            "/tmp/store",
            "--name",
            "users",
        ])
        .unwrap();
        match cli_parsed.command {
            Commands::CreateCollection(args) => {
                assert_eq!(args.store_path, "/tmp/store");
                assert_eq!(args.name, "users");
            },
            _ => panic!("Expected CreateCollection command"),
        }

        // Test insert command
        let cli_parsed = Cli::try_parse_from([
            "test",
            "insert",
            "--store-path",
            "/tmp/store",
            "--collection",
            "users",
            "--id",
            "user1",
            "--data",
            "{}",
        ])
        .unwrap();
        match cli_parsed.command {
            Commands::Insert(args) => {
                assert_eq!(args.store_path, "/tmp/store");
                assert_eq!(args.collection, "users");
                assert_eq!(args.id, "user1");
                assert_eq!(args.data, "{}");
            },
            _ => panic!("Expected Insert command"),
        }
    }

    /// Test CLI with verbose flag.
    ///
    /// This test checks that the verbose flag is parsed correctly.
    #[test]
    fn test_cli_verbose_parsing() {
        let cli_parsed = Cli::try_parse_from(["test", "-v", "init", "--path", "/tmp/store"]).unwrap();
        assert_eq!(cli_parsed.verbose, 1);

        let cli_parsed = Cli::try_parse_from(["test", "-vv", "init", "--path", "/tmp/store"]).unwrap();
        assert_eq!(cli_parsed.verbose, 2);
    }

    /// Test CLI with JSON flag.
    ///
    /// This test verifies that the JSON output flag is parsed correctly.
    #[test]
    fn test_cli_json_parsing() {
        let cli_parsed = Cli::try_parse_from(["test", "--json", "init", "--path", "/tmp/store"]).unwrap();
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

        // Create-collection without name
        let result = Cli::try_parse_from(["test", "create-collection", "--store-path", "/tmp"]);
        assert!(
            result.is_err(),
            "Create-collection should require name argument"
        );
    }

    /// Test run_command with Init command.
    ///
    /// This test verifies that run_command correctly dispatches to init::run.
    #[tokio::test]
    async fn test_run_command_init() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        let args = super::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        let cli = Cli {
            command:                  Commands::Init(args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
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
        let init_args = super::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        let init_cli = Cli {
            command:                  Commands::Init(init_args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };
        run_command(init_cli).await.unwrap();

        let args = super::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        let cli = Cli {
            command:                  Commands::CreateCollection(args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };

        let result = run_command(cli).await;
        assert!(
            result.is_ok(),
            "run_command should succeed for valid CreateCollection"
        );
    }

    /// Test run_command with Insert command.
    ///
    /// This test verifies that run_command correctly dispatches to insert::run.
    #[tokio::test]
    async fn test_run_command_insert() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let init_args = super::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        let init_cli = Cli {
            command:                  Commands::Init(init_args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };
        run_command(init_cli).await.unwrap();

        let create_args = super::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        let create_cli = Cli {
            command:                  Commands::CreateCollection(create_args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };
        run_command(create_cli).await.unwrap();

        let args = super::insert::InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id: "doc1".to_string(),
            data: r#"{"name": "Alice"}"#.to_string(),
            ..Default::default()
        };
        let cli = Cli {
            command:                  Commands::Insert(args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };

        let result = run_command(cli).await;
        assert!(
            result.is_ok(),
            "run_command should succeed for valid Insert"
        );
    }

    /// Test run_command with Get command.
    ///
    /// This test verifies that run_command correctly dispatches to get::run.
    #[tokio::test]
    async fn test_run_command_get() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let init_args = super::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        let init_cli = Cli {
            command:                  Commands::Init(init_args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };
        run_command(init_cli).await.unwrap();

        let create_args = super::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        let create_cli = Cli {
            command:                  Commands::CreateCollection(create_args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };
        run_command(create_cli).await.unwrap();

        let args = super::get::GetArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id: "doc1".to_string(),
            ..Default::default()
        };
        let cli = Cli {
            command:                  Commands::Get(args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };

        let result = run_command(cli).await;
        assert!(
            result.is_ok(),
            "run_command should succeed for Get (even if not found)"
        );
    }

    /// Test run_command with Update command.
    ///
    /// This test verifies that run_command correctly dispatches to update::run.
    #[tokio::test]
    async fn test_run_command_update() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let init_args = super::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        let init_cli = Cli {
            command:                  Commands::Init(init_args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };
        run_command(init_cli).await.unwrap();

        let create_args = super::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        let create_cli = Cli {
            command:                  Commands::CreateCollection(create_args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };
        run_command(create_cli).await.unwrap();

        let args = super::update::UpdateArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
            data:       r#"{"name": "Bob"}"#.to_string(),
        };
        let cli = Cli {
            command:                  Commands::Update(args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };

        let result = run_command(cli).await;
        assert!(result.is_ok(), "run_command should succeed for Update");
    }

    /// Test run_command with Delete command.
    ///
    /// This test verifies that run_command correctly dispatches to delete::run.
    #[tokio::test]
    async fn test_run_command_delete() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let init_args = super::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        let init_cli = Cli {
            command:                  Commands::Init(init_args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };
        run_command(init_cli).await.unwrap();

        let create_args = super::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        let create_cli = Cli {
            command:                  Commands::CreateCollection(create_args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };
        run_command(create_cli).await.unwrap();

        let args = super::delete::DeleteArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
        };
        let cli = Cli {
            command:                  Commands::Delete(args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };

        let result = run_command(cli).await;
        assert!(result.is_ok(), "run_command should succeed for Delete");
    }

    /// Test run_command with Generate command.
    ///
    /// This test verifies that run_command correctly dispatches to generate::run.
    #[tokio::test]
    async fn test_run_command_generate() {
        let args = super::generate::GenArgs {
            subcommand: super::generate::GenCommands::Key(super::generate::KeyArgs {
                key_type: super::generate::KeyType::Signing,
            }),
        };
        let cli = Cli {
            command:                  Commands::Generate(args),
            json:                     false,
            verbose:                  0,
            hash_algorithm:           "blake3".to_string(),
            signature_algorithm:      "ed25519".to_string(),
            encryption_algorithm:     "xchacha20poly1305".to_string(),
            key_derivation_algorithm: "argon2id".to_string(),
        };

        let result = run_command(cli).await;
        assert!(result.is_ok(), "run_command should succeed for Generate");
    }
}
