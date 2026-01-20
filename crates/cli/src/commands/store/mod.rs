use clap::{Args, Subcommand};

/// Arguments for the store command.
#[derive(Args)]
pub struct StoreArgs {
    #[command(subcommand)]
    /// The store subcommand to execute.
    pub subcommand: StoreCommands,
}

/// Enumeration of store subcommands.
#[derive(Subcommand)]
pub enum StoreCommands {
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
    /// List all collections in the store.
    ListCollections(ListCollectionsArgs),
    /// Delete a collection from the store.
    DeleteCollection(DeleteCollectionArgs),
}

/// Arguments for the store list-collections command.
#[derive(Args)]
pub struct ListCollectionsArgs {
    /// Path to the Sentinel store
    #[arg(short, long)]
    pub path:       String,
    /// Passphrase for decrypting the signing key
    #[arg(long)]
    pub passphrase: Option<String>,
}

/// Arguments for the store delete-collection command.
#[derive(Args)]
pub struct DeleteCollectionArgs {
    /// Path to the Sentinel store
    #[arg(short, long)]
    pub path:       String,
    /// Collection name
    #[arg(short, long)]
    pub collection: String,
    /// Passphrase for decrypting the signing key
    #[arg(long)]
    pub passphrase: Option<String>,
}

/// Run the store command.
///
/// # Arguments
/// * `args` - The parsed store command arguments.
///
/// # Returns
/// Returns `Ok(())` on success.
pub async fn run(args: StoreArgs) -> sentinel_dbms::Result<()> {
    match args.subcommand {
        StoreCommands::Init(init_args) => init::run(init_args).await,
        StoreCommands::Generate(gen_args) => generate::run(gen_args).await,
        StoreCommands::ListCollections(list_args) => run_list_collections(list_args).await,
        StoreCommands::DeleteCollection(delete_args) => run_delete_collection(delete_args).await,
    }
}

/// Run the store list-collections command.
pub async fn run_list_collections(args: ListCollectionsArgs) -> sentinel_dbms::Result<()> {
    let store = sentinel_dbms::Store::new_with_config(
        &args.path,
        args.passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let collections = store.list_collections().await?;
    for collection in collections {
        println!("{}", collection);
    }
    Ok(())
}

/// Run the store delete-collection command.
pub async fn run_delete_collection(args: DeleteCollectionArgs) -> sentinel_dbms::Result<()> {
    let store = sentinel_dbms::Store::new_with_config(
        &args.path,
        args.passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    store.delete_collection(&args.collection).await?;
    println!("Collection '{}' deleted successfully", args.collection);
    Ok(())
}

// Re-export submodules
pub mod generate;
pub mod init;
