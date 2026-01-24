//! Collection management commands.
//!
//! This module provides CLI commands for managing collections and their documents,
//! including CRUD operations, querying, and bulk operations.

use clap::{Args, Subcommand};

/// Collection management command arguments.
///
/// This struct defines the top-level arguments for collection operations.
#[derive(Args)]
pub struct CollectionArgs {
    /// Path to the Sentinel store
    #[arg(short, long)]
    pub store: String,

    /// Collection name
    #[arg(short, long)]
    pub name: String,

    /// Passphrase for decrypting signing key
    #[arg(long)]
    pub passphrase: Option<String>,

    #[command(subcommand)]
    pub command: CollectionCommands,
}

/// Collection subcommands.
///
/// These commands provide various collection management operations.
#[derive(Subcommand)]
pub enum CollectionCommands {
    /// Create a new collection within an existing store
    ///
    /// Collections are logical groupings of documents within a store.
    Create(create::CreateArgs),

    /// Insert a new document into a collection
    ///
    /// The document data must be valid JSON.
    Insert(insert::InsertArgs),

    /// Retrieve a document from a collection
    ///
    /// If the document exists, its JSON data is printed to stdout.
    Get(get::GetArgs),

    /// Update an existing document in a collection
    ///
    /// The entire document is replaced with the new data.
    Update(update::UpdateArgs),

    /// Insert or update a document in a collection
    ///
    /// Creates a new document if it doesn't exist, or updates it if it does.
    Upsert(upsert::UpsertArgs),

    /// Delete a document from a collection
    ///
    /// Performs a soft delete, moving the document to a .deleted subdirectory.
    Delete(delete::DeleteArgs),

    /// List all document IDs in a collection
    ///
    /// Prints the IDs of all documents in the specified collection.
    List(list::ListArgs),

    /// Query documents in a collection with filters and sorting
    ///
    /// Allows complex querying with filters, sorting, pagination, and projection.
    Query(query::QueryArgs),

    /// Count documents in a collection
    ///
    /// Returns the total number of documents in the specified collection.
    Count(count::CountArgs),

    /// Bulk insert documents into a collection
    ///
    /// Inserts multiple documents from a JSON file containing an array of document objects.
    #[command(name = "bulk-insert")]
    BulkInsert(bulk_insert::BulkInsertArgs),

    /// Get multiple documents by IDs
    ///
    /// Retrieves multiple documents from the collection by their IDs.
    #[command(name = "get-many")]
    GetMany(get_many::GetManyArgs),

    /// Aggregate documents in a collection
    ///
    /// Performs aggregation operations (count, sum, avg, min, max) on documents matching filters.
    Aggregate(aggregate::AggregateArgs),

    /// Show collection information and statistics
    ///
    /// Displays metadata and statistics for the collection.
    Info(info::InfoArgs),
}

mod aggregate;
mod bulk_insert;
mod count;
pub mod create;
mod delete;
mod get;
mod get_many;
mod info;
mod insert;
mod list;
mod query;
mod update;
mod upsert;

/// Execute collection command.
///
/// This function dispatches to the appropriate collection operation based on the subcommand.
pub async fn run(args: CollectionArgs) -> sentinel_dbms::Result<()> {
    match args.command {
        CollectionCommands::Create(sub_args) => create::run(args.store, args.name, args.passphrase, sub_args).await,
        CollectionCommands::Insert(sub_args) => insert::run(args.store, args.name, args.passphrase, sub_args).await,
        CollectionCommands::Get(sub_args) => get::run(args.store, args.name, args.passphrase, sub_args).await,
        CollectionCommands::Update(sub_args) => update::run(args.store, args.name, args.passphrase, sub_args).await,
        CollectionCommands::Upsert(sub_args) => upsert::run(args.store, args.name, args.passphrase, sub_args).await,
        CollectionCommands::Delete(sub_args) => delete::run(args.store, args.name, args.passphrase, sub_args).await,
        CollectionCommands::List(sub_args) => list::run(args.store, args.name, args.passphrase, sub_args).await,
        CollectionCommands::Query(sub_args) => query::run(args.store, args.name, args.passphrase, sub_args).await,
        CollectionCommands::Count(sub_args) => count::run(args.store, args.name, args.passphrase, sub_args).await,
        CollectionCommands::BulkInsert(sub_args) => {
            bulk_insert::run(args.store, args.name, args.passphrase, sub_args).await
        },
        CollectionCommands::GetMany(sub_args) => get_many::run(args.store, args.name, args.passphrase, sub_args).await,
        CollectionCommands::Aggregate(sub_args) => {
            aggregate::run(args.store, args.name, args.passphrase, sub_args).await
        },
        CollectionCommands::Info(sub_args) => info::run(args.store, args.name, args.passphrase, sub_args).await,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn test_run_create_collection() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();
        let collection_name = "test_collection";

        // First create the store
        sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let args = CollectionArgs {
            store:      store_path,
            name:       collection_name.to_string(),
            passphrase: None,
            command:    CollectionCommands::Create(create::CreateArgs::default()),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_insert() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();
        let collection_name = "test_collection";

        let args = CollectionArgs {
            store:      store_path,
            name:       collection_name.to_string(),
            passphrase: None,
            command:    CollectionCommands::Insert(insert::InsertArgs {
                id:   Some("doc1".to_string()),
                data: Some(r#"{"name": "Alice"}"#.to_string()),
                bulk: None,
                wal:  crate::commands::WalArgs::default(),
            }),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_get() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();
        let collection_name = "test_collection";

        // First create store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        let collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        // Insert a document
        collection
            .insert("doc1", serde_json::json!({"name": "Alice"}))
            .await
            .unwrap();

        let args = CollectionArgs {
            store:      store_path,
            name:       collection_name.to_string(),
            passphrase: None,
            command:    CollectionCommands::Get(get::GetArgs {
                id:               "doc1".to_string(),
                verify_signature: true,
                verify_hash:      true,
                signature_mode:   "strict".to_string(),
                empty_sig_mode:   "warn".to_string(),
                hash_mode:        "strict".to_string(),
                wal:              crate::commands::WalArgs::default(),
            }),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_get_many() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();
        let collection_name = "test_collection";

        let args = CollectionArgs {
            store:      store_path,
            name:       collection_name.to_string(),
            passphrase: None,
            command:    CollectionCommands::GetMany(get_many::GetManyArgs {
                ids:    vec!["doc1".to_string()],
                format: "json".to_string(),
                wal:    crate::commands::WalArgs::default(),
            }),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_list() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();
        let collection_name = "test_collection";

        // First create store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        let collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        // Insert a document
        collection
            .insert("doc1", serde_json::json!({"name": "Alice"}))
            .await
            .unwrap();

        let args = CollectionArgs {
            store:      store_path,
            name:       collection_name.to_string(),
            passphrase: None,
            command:    CollectionCommands::List(list::ListArgs {
                verify_signature: true,
                verify_hash:      true,
                signature_mode:   "strict".to_string(),
                empty_sig_mode:   "warn".to_string(),
                hash_mode:        "strict".to_string(),
                wal:              crate::commands::WalArgs::default(),
            }),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_info() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();
        let collection_name = "test_collection";

        let args = CollectionArgs {
            store:      store_path,
            name:       collection_name.to_string(),
            passphrase: None,
            command:    CollectionCommands::Info(info::InfoArgs {
                format: "table".to_string(),
            }),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_query() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();
        let collection_name = "test_collection";

        let args = CollectionArgs {
            store:      store_path,
            name:       collection_name.to_string(),
            passphrase: None,
            command:    CollectionCommands::Query(query::QueryArgs {
                verify_signature: true,
                verify_hash:      true,
                signature_mode:   "strict".to_string(),
                empty_sig_mode:   "warn".to_string(),
                hash_mode:        "strict".to_string(),
                filter:           vec![],
                sort:             None,
                limit:            None,
                offset:           None,
                project:          None,
                format:           "json".to_string(),
                wal:              crate::commands::WalArgs::default(),
            }),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_upsert() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();
        let collection_name = "test_collection";

        let args = CollectionArgs {
            store:      store_path,
            name:       collection_name.to_string(),
            passphrase: None,
            command:    CollectionCommands::Upsert(upsert::UpsertArgs {
                id:   "doc1".to_string(),
                data: r#"{"name": "Alice"}"#.to_string(),
                wal:  crate::commands::WalArgs::default(),
            }),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }
}
