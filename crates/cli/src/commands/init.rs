use clap::Args;

/// Arguments for the init command.
#[derive(Args)]
pub struct InitArgs {
    /// Path to the store directory
    #[arg(short, long)]
    pub path: String,
}

use std::io;

use tracing::{error, info};

/// Initialize a new Sentinel store at the specified path.
///
/// This function creates the necessary directory structure and metadata
/// for a new Sentinel store. It logs the operation and handles any errors
/// that may occur during initialization.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for init.
///
/// # Returns
/// Returns `Ok(())` on success, or an `io::Error` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::init::{run, InitArgs};
///
/// let args = InitArgs {
///     path: "/tmp/my_store".to_string(),
/// };
/// run(args).await?;
/// ```
pub async fn run(args: InitArgs) -> io::Result<()> {
    let path = args.path;
    info!("Initializing store at {}", path);
    match sentinel::Store::new(&path).await {
        Ok(_) => {
            info!("Store initialized successfully at {}", path);
            Ok(())
        },
        Err(e) => {
            error!("Failed to initialize store at {}: {}", path, e);
            Err(e)
        },
    }
}
