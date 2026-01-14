use clap::Parser as _;

use crate::{
    commands::{run_command, Cli},
    logging::init_tracing,
};

/// Run the Sentinel CLI application.
///
/// This is the main entry point for the CLI. It parses command-line arguments,
/// initializes tracing, and executes the requested command.
///
/// # Returns
/// Returns `Ok(())` on successful execution, or a `SentinelError` on failure.
pub async fn run() -> sentinel::Result<()> {
    let cli = Cli::parse();

    init_tracing(cli.json, cli.verbose);

    run_command(cli.command).await
}
