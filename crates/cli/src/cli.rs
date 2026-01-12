use crate::commands::{run_command, Cli};
use crate::logging::init_tracing;
use clap::Parser;
use std::io;

/// Run the Sentinel CLI application.
///
/// This is the main entry point for the CLI. It parses command-line arguments,
/// initializes tracing, and executes the requested command.
///
/// # Returns
/// Returns `Ok(())` on successful execution, or an `io::Error` on failure.
pub async fn run() -> io::Result<()> {
    let cli = Cli::parse();

    init_tracing(cli.json, cli.verbose);

    run_command(cli.command).await
}