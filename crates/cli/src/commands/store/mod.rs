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
    List(list::ListCollectionsArgs),
    /// Delete a collection from the store.
    Delete(delete::DeleteCollectionArgs),
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
        StoreCommands::List(list_args) => list::run(list_args).await,
        StoreCommands::Delete(delete_args) => delete::run(delete_args).await,
    }
}

// Re-export submodules
pub mod delete;
pub mod generate;
pub mod init;
pub mod list;
