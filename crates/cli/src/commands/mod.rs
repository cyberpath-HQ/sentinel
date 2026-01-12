use clap::{Parser, Subcommand};

/// Command handlers for the Sentinel CLI.
///
/// This module contains submodules for each CLI command, each implementing
/// the logic for a specific operation on the Sentinel DBMS.
mod create_collection;
mod delete;
mod get;
mod init;
mod insert;
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
pub async fn run_command(command: Commands) -> std::io::Result<()> {
    match command {
        Commands::Init(args) => init::run(args).await,
        Commands::CreateCollection(args) => create_collection::run(args).await,
        Commands::Insert(args) => insert::run(args).await,
        Commands::Get(args) => get::run(args).await,
        Commands::Update(args) => update::run(args).await,
        Commands::Delete(args) => delete::run(args).await,
    }
}
