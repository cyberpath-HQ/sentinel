use clap::{Parser, Subcommand};
use serde_json::Value;
use tracing::{error, info, warn};

/// The main CLI structure for the Sentinel document DBMS.
///
/// This CLI provides commands to interact with Sentinel stores, collections, and documents.
/// It supports various operations like initializing stores, managing collections, and CRUD operations on documents.
///
/// # Examples
///
/// Initialize a new store:
/// ```bash
/// sentinel-cli init /path/to/store
/// ```
///
/// Insert a document:
/// ```bash
/// sentinel-cli insert /path/to/store my_collection doc1 '{"key": "value"}'
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
    /// This command creates the necessary directory structure and metadata for a new Sentinel store.
    ///
    /// # Arguments
    /// * `path` - The filesystem path where the store should be created.
    Init {
        /// Path to the store directory
        path: String,
    },
    /// Create a new collection within an existing store.
    ///
    /// Collections are logical groupings of documents within a store.
    ///
    /// # Arguments
    /// * `store_path` - Path to the existing store.
    /// * `name` - Name of the collection to create.
    CreateCollection {
        /// Store path
        store_path: String,
        /// Collection name
        name: String,
    },
    /// Insert a new document into a collection.
    ///
    /// The document data must be valid JSON.
    ///
    /// # Arguments
    /// * `store_path` - Path to the store.
    /// * `collection` - Name of the collection.
    /// * `id` - Unique identifier for the document.
    /// * `data` - JSON string representing the document data.
    Insert {
        /// Store path
        store_path: String,
        /// Collection name
        collection: String,
        /// Document ID
        id: String,
        /// JSON data (as string)
        data: String,
    },
    /// Retrieve a document from a collection.
    ///
    /// If the document exists, its JSON data is printed to stdout.
    ///
    /// # Arguments
    /// * `store_path` - Path to the store.
    /// * `collection` - Name of the collection.
    /// * `id` - Document ID to retrieve.
    Get {
        /// Store path
        store_path: String,
        /// Collection name
        collection: String,
        /// Document ID
        id: String,
    },
    /// Update an existing document in a collection.
    ///
    /// The entire document is replaced with the new data.
    ///
    /// # Arguments
    /// * `store_path` - Path to the store.
    /// * `collection` - Name of the collection.
    /// * `id` - Document ID to update.
    /// * `data` - New JSON data for the document.
    Update {
        /// Store path
        store_path: String,
        /// Collection name
        collection: String,
        /// Document ID
        id: String,
        /// JSON data (as string)
        data: String,
    },
    /// Delete a document from a collection.
    ///
    /// # Arguments
    /// * `store_path` - Path to the store.
    /// * `collection` - Name of the collection.
    /// * `id` - Document ID to delete.
    Delete {
        /// Store path
        store_path: String,
        /// Collection name
        collection: String,
        /// Document ID
        id: String,
    },
}

/// Execute the specified CLI command.
///
/// This function handles the logic for each command, performing the appropriate operations
/// on the Sentinel store and logging the results.
///
/// # Arguments
/// * `command` - The command to execute.
///
/// # Returns
/// Returns `Ok(())` on success, or an `io::Error` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::{Commands, run_command};
///
/// let cmd = Commands::Init { path: "/tmp/store".to_string() };
/// run_command(cmd).await?;
/// ```
pub async fn run_command(command: Commands) -> std::io::Result<()> {
    match command {
        Commands::Init { path } => {
            info!("Initializing store at {}", path);
            match sentinel::Store::new(&path).await {
                Ok(_) => info!("Store initialized successfully at {}", path),
                Err(e) => {
                    error!("Failed to initialize store at {}: {}", path, e);
                    return Err(e);
                }
            }
        }
        Commands::CreateCollection { store_path, name } => {
            info!("Creating collection '{}' in store {}", name, store_path);
            let store = sentinel::Store::new(&store_path).await?;
            match store.collection(&name).await {
                Ok(_) => info!("Collection '{}' created successfully", name),
                Err(e) => {
                    error!("Failed to create collection '{}' in store {}: {}", name, store_path, e);
                    return Err(e);
                }
            }
        }
        Commands::Insert { store_path, collection, id, data } => {
            info!("Inserting document '{}' into collection '{}' in store {}", id, collection, store_path);
            let store = sentinel::Store::new(&store_path).await?;
            let coll = store.collection(&collection).await?;
            let value: Value = match serde_json::from_str(&data) {
                Ok(v) => v,
                Err(e) => {
                    error!("Invalid JSON data: {}", e);
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, e));
                }
            };
            match coll.insert(&id, value).await {
                Ok(_) => info!("Document '{}' inserted successfully", id),
                Err(e) => {
                    error!("Failed to insert document '{}' into collection '{}' in store {}: {}", id, collection, store_path, e);
                    return Err(e);
                }
            }
        }
        Commands::Get { store_path, collection, id } => {
            info!("Getting document '{}' from collection '{}' in store {}", id, collection, store_path);
            let store = sentinel::Store::new(&store_path).await?;
            let coll = store.collection(&collection).await?;
            match coll.get(&id).await {
                Ok(Some(doc)) => {
                    info!("Document '{}' retrieved successfully", id);
                    println!("{}", serde_json::to_string_pretty(&doc.data)?);
                }
                Ok(None) => {
                    warn!("Document '{}' not found", id);
                }
                Err(e) => {
                    error!("Failed to get document '{}' from collection '{}' in store {}: {}", id, collection, store_path, e);
                    return Err(e);
                }
            }
        }
        Commands::Update { store_path, collection, id, data } => {
            info!("Updating document '{}' in collection '{}' in store {}", id, collection, store_path);
            let store = sentinel::Store::new(&store_path).await?;
            let coll = store.collection(&collection).await?;
            let value: Value = match serde_json::from_str(&data) {
                Ok(v) => v,
                Err(e) => {
                    error!("Invalid JSON data: {}", e);
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, e));
                }
            };
            match coll.update(&id, value).await {
                Ok(_) => info!("Document '{}' updated successfully", id),
                Err(e) => {
                    error!("Failed to update document '{}' in collection '{}' in store {}: {}", id, collection, store_path, e);
                    return Err(e);
                }
            }
        }
        Commands::Delete { store_path, collection, id } => {
            info!("Deleting document '{}' from collection '{}' in store {}", id, collection, store_path);
            let store = sentinel::Store::new(&store_path).await?;
            let coll = store.collection(&collection).await?;
            match coll.delete(&id).await {
                Ok(_) => info!("Document '{}' deleted successfully", id),
                Err(e) => {
                    error!("Failed to delete document '{}' from collection '{}' in store {}: {}", id, collection, store_path, e);
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}