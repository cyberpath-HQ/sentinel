//! WAL (Write-Ahead Logging) management commands.
//!
//! This module provides CLI commands for managing Write-Ahead Logging operations
//! including checkpointing, verification, recovery, and configuration management.

use clap::{Args, Subcommand};

/// WAL management command arguments.
///
/// This struct defines the top-level arguments for WAL operations.
#[derive(Args)]
pub struct WalArgs {
    /// Path to the Sentinel store
    #[arg(short, long)]
    pub store_path: String,

    /// Collection name (optional, for collection-specific operations)
    #[arg(short, long)]
    pub collection: Option<String>,

    /// WAL subcommand
    #[command(subcommand)]
    pub command: WalCommands,
}

/// WAL subcommands.
///
/// These commands provide various WAL management operations.
#[derive(Subcommand)]
pub enum WalCommands {
    /// Create a durable recovery point by ensuring all WAL entries are written to disk
    ///
    /// This operation flushes any buffered writes and synchronizes the WAL file to disk,
    /// creating a safe recovery point without removing any log data.
    Checkpoint(checkpoint::CheckpointArgs),

    /// Verify WAL integrity and consistency with current document state
    ///
    /// Checks that WAL entries are valid and match the current state of documents,
    /// reporting any inconsistencies or corruption issues.
    Verify(verify::VerifyArgs),

    /// Restore data consistency by replaying WAL entries after a failure
    ///
    /// Replays logged operations from the WAL to recover any lost changes
    /// and ensure data consistency following an unexpected shutdown or crash.
    Recover(recover::RecoverArgs),

    /// Display WAL entries in chronological order
    ///
    /// Lists WAL entries with details about operations, timestamps, and affected documents.
    List(list::ListArgs),

    /// Show WAL file statistics and metrics
    ///
    /// Displays information about WAL file size, entry counts, and performance metrics.
    Stats(stats::StatsArgs),
}

mod checkpoint;
mod list;
mod recover;
mod stats;
mod verify;

/// Execute WAL command.
///
/// This function dispatches to the appropriate WAL operation based on the subcommand.
pub async fn run(args: WalArgs) -> sentinel_dbms::Result<()> {
    match args.command {
        WalCommands::Checkpoint(sub_args) => checkpoint::run(args.store_path, args.collection, sub_args).await,
        WalCommands::Verify(sub_args) => verify::run(args.store_path, args.collection, sub_args).await,
        WalCommands::Recover(sub_args) => recover::run(args.store_path, args.collection, sub_args).await,
        WalCommands::List(sub_args) => list::run(args.store_path, args.collection, sub_args).await,
        WalCommands::Stats(sub_args) => stats::run(args.store_path, args.collection, sub_args).await,
    }
}
