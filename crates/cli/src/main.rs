use tracing::error;

/// CLI module for command-line interface logic.
mod cli;
/// Commands module for subcommand implementations.
mod commands;
/// Logging module for setting up tracing.
mod logging;

#[tokio::main]
async fn main() {
    if let Err(e) = cli::run().await {
        error!("{}", e);
        std::process::exit(1);
    }
}
