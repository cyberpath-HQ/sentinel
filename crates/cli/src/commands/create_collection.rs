use clap::Args;

/// Arguments for the create-collection command.
#[derive(Args)]
pub struct CreateCollectionArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path: String,
    /// Collection name
    #[arg(short, long)]
    pub name: String,
}

use std::io;
use tracing::{error, info};

/// Create a new collection within an existing Sentinel store.
///
/// This function creates a logical grouping for documents within the specified store.
/// It validates that the store exists and handles any errors during collection creation.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for create-collection.
///
/// # Returns
/// Returns `Ok(())` on success, or an `io::Error` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::create_collection::{CreateCollectionArgs, run};
///
/// let args = CreateCollectionArgs {
///     store_path: "/tmp/my_store".to_string(),
///     name: "users".to_string(),
/// };
/// run(args).await?;
/// ```
pub async fn run(args: CreateCollectionArgs) -> io::Result<()> {
    let store_path = args.store_path;
    let name = args.name;
    info!("Creating collection '{}' in store {}", name, store_path);
    let store = sentinel::Store::new(&store_path).await?;
    match store.collection(&name).await {
        Ok(_) => {
            info!("Collection '{}' created successfully", name);
            Ok(())
        }
        Err(e) => {
            error!("Failed to create collection '{}' in store {}: {}", name, store_path, e);
            Err(e)
        }
    }
}