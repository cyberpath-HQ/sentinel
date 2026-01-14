use clap::{Parser, Subcommand};

/// Command handlers for the Sentinel CLI.
///
/// This module contains submodules for each CLI command, each implementing
/// the logic for a specific operation on the Sentinel DBMS.
/// Create collection command module.
mod create_collection;
/// Delete command module.
mod delete;
/// Generate key command module.
mod generate_key;
/// Get command module.
mod get;
/// Init command module.
mod init;
/// Insert command module.
mod insert;
/// Update command module.
mod update;

/// The main CLI structure for the Sentinel document DBMS.
///
/// This CLI provides commands to interact with Sentinel stores, collections, and documents.
/// It supports various operations like initializing stores, managing collections, and CRUD
/// operations on documents.
///
/// # Examples
///
/// Initialize a new store:
/// ```bash
/// sentinel-cli init --path /path/to/store
/// ```
///
/// Insert a document:
/// ```bash
/// sentinel-cli insert --store-path /path/to/store --collection my_collection --id doc1 --data '{"key": "value"}'
/// ```
#[derive(Parser)]
#[command(name = "sentinel-cli")]
#[command(about = "A document DBMS CLI")]
pub struct Cli {
    #[command(subcommand)]
    /// The subcommand to execute.
    pub command: Commands,

    /// Output logs in JSON format
    #[arg(long)]
    pub json: bool,

    /// Increase verbosity (can be used multiple times: -v for debug, -vv for trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
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
    ///
    /// # Arguments
    /// * `--path` - The filesystem path where the store should be created.
    Init(init::InitArgs),
    /// Generate a new signing key.
    ///
    /// This command generates a new Ed25519 signing key and outputs it as a hex string.
    /// The key can be used for signing documents.
    GenerateKey(generate_key::GenerateKeyArgs),
    /// Create a new collection within an existing store.
    ///
    /// Collections are logical groupings of documents within a store.
    ///
    /// # Arguments
    /// * `--store-path` - Path to the existing store.
    /// * `--name` - Name of the collection to create.
    CreateCollection(create_collection::CreateCollectionArgs),
    /// Insert a new document into a collection.
    ///
    /// The document data must be valid JSON.
    ///
    /// # Arguments
    /// * `--store-path` - Path to the store.
    /// * `--collection` - Name of the collection.
    /// * `--id` - Unique identifier for the document.
    /// * `--data` - JSON string representing the document data.
    Insert(insert::InsertArgs),
    /// Retrieve a document from a collection.
    ///
    /// If the document exists, its JSON data is printed to stdout.
    ///
    /// # Arguments
    /// * `--store-path` - Path to the store.
    /// * `--collection` - Name of the collection.
    /// * `--id` - Document ID to retrieve.
    Get(get::GetArgs),
    /// Update an existing document in a collection.
    ///
    /// The entire document is replaced with the new data.
    ///
    /// # Arguments
    /// * `--store-path` - Path to the store.
    /// * `--collection` - Name of the collection.
    /// * `--id` - Document ID to update.
    /// * `--data` - New JSON data for the document.
    Update(update::UpdateArgs),
    /// Delete a document from a collection.
    ///
    /// # Arguments
    /// * `--store-path` - Path to the store.
    /// * `--collection` - Name of the collection.
    /// * `--id` - Document ID to delete.
    Delete(delete::DeleteArgs),
}

/// Execute the specified CLI command.
///
/// This function dispatches to the appropriate command handler based on the
/// provided command variant, delegating the actual work to isolated modules.
///
/// # Arguments
/// * `command` - The command to execute.
///
/// # Returns
/// Returns `Ok(())` on success, or an `io::Error` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::{run_command, Commands};
///
/// let cmd = Commands::Init(init::InitArgs {
///     path: "/tmp/store".to_string(),
/// });
/// run_command(cmd).await?;
/// ```
pub async fn run_command(command: Commands) -> sentinel::Result<()> {
    match command {
        Commands::Init(args) => init::run(args).await,
        Commands::GenerateKey(args) => generate_key::run(args).await,
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
        };
        let command = Commands::Init(args);

        let result = run_command(command).await;
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
        };
        run_command(Commands::Init(init_args)).await.unwrap();

        let args = super::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name:       "test_collection".to_string(),
        };
        let command = Commands::CreateCollection(args);

        let result = run_command(command).await;
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
        };
        run_command(Commands::Init(init_args)).await.unwrap();

        let create_args = super::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name:       "test_collection".to_string(),
        };
        run_command(Commands::CreateCollection(create_args))
            .await
            .unwrap();

        let args = super::insert::InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
            data:       r#"{"name": "Alice"}"#.to_string(),
        };
        let command = Commands::Insert(args);

        let result = run_command(command).await;
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
        };
        run_command(Commands::Init(init_args)).await.unwrap();

        let create_args = super::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name:       "test_collection".to_string(),
        };
        run_command(Commands::CreateCollection(create_args))
            .await
            .unwrap();

        let args = super::get::GetArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
        };
        let command = Commands::Get(args);

        let result = run_command(command).await;
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
        };
        run_command(Commands::Init(init_args)).await.unwrap();

        let create_args = super::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name:       "test_collection".to_string(),
        };
        run_command(Commands::CreateCollection(create_args))
            .await
            .unwrap();

        let args = super::update::UpdateArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
            data:       r#"{"name": "Bob"}"#.to_string(),
        };
        let command = Commands::Update(args);

        let result = run_command(command).await;
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
        };
        run_command(Commands::Init(init_args)).await.unwrap();

        let create_args = super::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name:       "test_collection".to_string(),
        };
        run_command(Commands::CreateCollection(create_args))
            .await
            .unwrap();

        let args = super::delete::DeleteArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
        };
        let command = Commands::Delete(args);

        let result = run_command(command).await;
        assert!(result.is_ok(), "run_command should succeed for Delete");
    }
}
