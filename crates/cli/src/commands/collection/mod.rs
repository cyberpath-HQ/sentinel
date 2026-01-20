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
