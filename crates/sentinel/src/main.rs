use clap::{Parser, Subcommand};
use sentinel::{Store, Collection};
use serde_json::Value;
use std::io::{self, Write};
use tokio;

#[derive(Parser)]
#[command(name = "sentinel-cli")]
#[command(about = "A simple document DBMS CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new store
    Init {
        /// Path to the store directory
        path: String,
    },
    /// Create a collection
    CreateCollection {
        /// Store path
        store_path: String,
        /// Collection name
        name: String,
    },
    /// Insert a document
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
    /// Get a document
    Get {
        /// Store path
        store_path: String,
        /// Collection name
        collection: String,
        /// Document ID
        id: String,
    },
    /// Update a document
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
    /// Delete a document
    Delete {
        /// Store path
        store_path: String,
        /// Collection name
        collection: String,
        /// Document ID
        id: String,
    },
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            Store::new(&path).await?;
            println!("Store initialized at {}", path);
        }
        Commands::CreateCollection { store_path, name } => {
            let store = Store::new(&store_path).await?;
            store.collection(&name).await?;
            println!("Collection '{}' created", name);
        }
        Commands::Insert { store_path, collection, id, data } => {
            let store = Store::new(&store_path).await?;
            let coll = store.collection(&collection).await?;
            let value: Value = serde_json::from_str(&data)?;
            coll.insert(&id, value).await?;
            println!("Document '{}' inserted", id);
        }
        Commands::Get { store_path, collection, id } => {
            let store = Store::new(&store_path).await?;
            let coll = store.collection(&collection).await?;
            if let Some(doc) = coll.get(&id).await? {
                println!("{}", serde_json::to_string_pretty(&doc.data)?);
            } else {
                println!("Document not found");
            }
        }
        Commands::Update { store_path, collection, id, data } => {
            let store = Store::new(&store_path).await?;
            let coll = store.collection(&collection).await?;
            let value: Value = serde_json::from_str(&data)?;
            coll.update(&id, value).await?;
            println!("Document '{}' updated", id);
        }
        Commands::Delete { store_path, collection, id } => {
            let store = Store::new(&store_path).await?;
            let coll = store.collection(&collection).await?;
            coll.delete(&id).await?;
            println!("Document '{}' deleted", id);
        }
    }

    Ok(())
}