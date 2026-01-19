//! WAL (Write-Ahead Logging) management commands.
//!
//! This module provides CLI commands for managing Write-Ahead Logging operations
//! including checkpointing, verification, recovery, and configuration management.

use clap::{Args, Subcommand};
use serde_json::json;
use sentinel_dbms::futures::StreamExt;

/// WAL management command arguments.
///
/// This struct defines the top-level arguments for WAL operations.
#[derive(Args)]
pub struct WalArgs {
    /// Path to the Sentinel store
    #[arg(short, long)]
    pub store_path: String,

    /// Collection name (required for collection-specific operations)
    #[arg(short, long)]
    pub collection: Option<String>,

    #[command(subcommand)]
    pub command: WalCommands,
}

/// WAL subcommands.
///
/// These commands provide various WAL management operations.
#[derive(Subcommand)]
pub enum WalCommands {
    /// Checkpoint WAL entries to main data store
    ///
    /// Flushes pending WAL entries and truncates the log file.
    /// Use --collection to checkpoint a specific collection, or omit to checkpoint all collections.
    Checkpoint,

    /// Verify WAL consistency
    ///
    /// Checks that WAL entries match the current state of documents.
    /// Use --collection to verify a specific collection, or omit to verify all collections.
    Verify,

    /// Recover from WAL
    ///
    /// Replays WAL entries to restore data consistency after a crash.
    /// Use --collection to recover a specific collection, or omit to recover all collections.
    Recover,

    /// List WAL entries
    ///
    /// Displays WAL entries in chronological order.
    /// Use --collection to list entries for a specific collection, or omit to list all collections.
    List {
        /// Maximum number of entries to display
        #[arg(short, long, default_value = "50")]
        limit: usize,

        /// Output format (json, table)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show WAL statistics
    ///
    /// Displays WAL file size, entry count, and other metrics.
    /// Use --collection to show stats for a specific collection, or omit to show store-wide stats.
    Stats,
}

/// Execute WAL command.
///
/// This function dispatches to the appropriate WAL operation based on the subcommand.
pub async fn run(args: WalArgs) -> sentinel_dbms::Result<()> {
    let store = sentinel_dbms::Store::new(&args.store_path, None).await?;

    match args.command {
        WalCommands::Checkpoint => run_checkpoint(&store, args.collection).await,
        WalCommands::Verify => run_verify(&store, args.collection).await,
        WalCommands::Recover => run_recover(&store, args.collection).await,
        WalCommands::List {
            limit,
            format,
        } => run_list(&store, args.collection, limit, &format).await,
        WalCommands::Stats => run_stats(&store, args.collection).await,
    }
}

/// Run WAL checkpoint operation.
async fn run_checkpoint(store: &sentinel_dbms::Store, collection: Option<String>) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::{CollectionWalOps, StoreWalOps};

    if let Some(collection_name) = collection {
        let collection = store.collection(&collection_name).await?;
        println!("Checkpointing WAL for collection '{}'...", collection_name);
        collection.checkpoint_wal().await?;
        println!(
            "✓ WAL checkpoint completed for collection '{}'",
            collection_name
        );
    }
    else {
        println!("Checkpointing WAL for all collections...");
        store.checkpoint_all_collections().await?;
        println!("✓ WAL checkpoint completed for all collections");
    }

    Ok(())
}

/// Run WAL verification operation.
async fn run_verify(store: &sentinel_dbms::Store, collection: Option<String>) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::{CollectionWalOps, StoreWalOps};

    if let Some(collection_name) = collection {
        let collection = store.collection(&collection_name).await?;
        println!("Verifying WAL for collection '{}'...", collection_name);

        let result = collection.verify_against_wal().await?;
        println!(
            "✓ Verification completed for collection '{}'",
            collection_name
        );
        println!("  Entries processed: {}", result.entries_processed);
        println!("  Passed: {}", result.passed);

        if !result.issues.is_empty() {
            println!("  Issues found: {}", result.issues.len());
            for issue in &result.issues {
                println!("    - {}", issue.description);
            }
            if result.issues.iter().any(|i| i.is_critical) {
                return Err(sentinel_dbms::SentinelError::ConfigError {
                    message: format!(
                        "Critical verification issues found in collection '{}'",
                        collection_name
                    ),
                });
            }
        }
    }
    else {
        println!("Verifying WAL for all collections...");
        let issues = store.verify_all_collections().await?;

        if issues.is_empty() {
            println!("✓ All collections verified successfully");
        }
        else {
            println!("✗ Verification issues found:");
            let mut has_critical = false;
            for (collection_name, collection_issues) in issues {
                println!(
                    "  Collection '{}': {} issues",
                    collection_name,
                    collection_issues.len()
                );
                for issue in collection_issues {
                    println!("    - {}", issue.description);
                    if issue.is_critical {
                        has_critical = true;
                    }
                }
            }
            if has_critical {
                return Err(sentinel_dbms::SentinelError::ConfigError {
                    message: "Critical verification issues found".to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Run WAL recovery operation.
async fn run_recover(store: &sentinel_dbms::Store, collection: Option<String>) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::{CollectionWalOps, StoreWalOps};

    if let Some(collection_name) = collection {
        let collection = store.collection(&collection_name).await?;
        println!(
            "Recovering from WAL for collection '{}'...",
            collection_name
        );

        let result = collection.recover_from_wal().await?;
        println!("✓ Recovery completed for collection '{}'", collection_name);
        println!("  Operations recovered: {}", result.recovered_operations);
        println!("  Operations skipped: {}", result.skipped_operations);

        if result.failed_operations > 0 {
            println!("  Operations failed: {}", result.failed_operations);
            for failure in &result.failures {
                println!("    - {:?}", failure);
            }
        }
    }
    else {
        println!("Recovering from WAL for all collections...");
        let recovery_stats = store.recover_all_collections().await?;

        let total_operations: usize = recovery_stats.values().sum();
        println!(
            "✓ Recovery completed for {} collections",
            recovery_stats.len()
        );
        println!("  Total operations recovered: {}", total_operations);

        for (collection_name, count) in recovery_stats {
            println!("  Collection '{}': {} operations", collection_name, count);
        }
    }

    Ok(())
}

/// Run WAL list operation.
async fn run_list(
    store: &sentinel_dbms::Store,
    collection: Option<String>,
    limit: usize,
    format: &str,
) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::{CollectionWalOps, StoreWalOps};

    if let Some(collection_name) = collection {
        let collection = store.collection(&collection_name).await?;
        println!(
            "Listing WAL entries for collection '{}' (limit: {})...",
            collection_name, limit
        );

        let mut stream = collection.stream_wal_entries().await?;
        let mut count = 0;

        while let Some(result) = stream.next().await {
            if count >= limit {
                println!("... (truncated, showing first {} entries)", limit);
                break;
            }

            let entry = result?;
            count += 1;

            match format {
                "json" => {
                    let json_entry = json!({
                        "entry_type": format!("{:?}", entry.entry_type),
                        "transaction_id": entry.transaction_id_str(),
                        "collection": entry.collection_str(),
                        "document_id": entry.document_id_str(),
                        "timestamp": entry.timestamp,
                        "data_length": entry.data.as_ref().map(|s| s.len()).unwrap_or(0),
                        "has_data": entry.data.is_some()
                    });
                    println!("{}", serde_json::to_string_pretty(&json_entry)?);
                },
                "table" => {
                    println!(
                        "{:>3} | {:<8} | {:<12} | {:<10} | {}",
                        count,
                        format!("{:?}", entry.entry_type),
                        entry.transaction_id_str(),
                        entry.document_id_str(),
                        chrono::DateTime::from_timestamp(entry.timestamp as i64, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_else(|| "invalid timestamp".to_string())
                    );
                },
                _ => {
                    return Err(sentinel_dbms::SentinelError::ConfigError {
                        message: format!("Unsupported format: {}", format),
                    });
                },
            }
        }

        println!("Total entries shown: {}", count);
    }
    else {
        println!(
            "Listing WAL entries for all collections (limit: {})...",
            limit
        );

        let mut stream = store.stream_all_wal_entries().await?;
        let mut count = 0;

        while let Some(result) = stream.next().await {
            if count >= limit {
                println!("... (truncated, showing first {} entries)", limit);
                break;
            }

            let (collection_name, entry) = result?;
            count += 1;

            match format {
                "json" => {
                    let json_entry = json!({
                        "collection": collection_name,
                        "entry_type": format!("{:?}", entry.entry_type),
                        "transaction_id": entry.transaction_id_str(),
                        "document_id": entry.document_id_str(),
                        "timestamp": entry.timestamp,
                        "data_length": entry.data.as_ref().map(|s| s.len()).unwrap_or(0),
                        "has_data": entry.data.is_some()
                    });
                    println!("{}", serde_json::to_string_pretty(&json_entry)?);
                },
                "table" => {
                    println!(
                        "{:>3} | {:<15} | {:<8} | {:<12} | {:<10} | {}",
                        count,
                        collection_name,
                        format!("{:?}", entry.entry_type),
                        entry.transaction_id_str(),
                        entry.document_id_str(),
                        chrono::DateTime::from_timestamp(entry.timestamp as i64, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_else(|| "invalid timestamp".to_string())
                    );
                },
                _ => {
                    return Err(sentinel_dbms::SentinelError::ConfigError {
                        message: format!("Unsupported format: {}", format),
                    });
                },
            }
        }

        println!("Total entries shown: {}", count);
    }

    Ok(())
}

/// Run WAL stats operation.
async fn run_stats(store: &sentinel_dbms::Store, collection: Option<String>) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::CollectionWalOps;

    if let Some(collection_name) = collection {
        let collection = store.collection(&collection_name).await?;

        let size = collection.wal_size().await?;
        let count = collection.wal_entries_count().await?;

        println!("WAL Statistics for collection '{}':", collection_name);
        println!(
            "  Size: {} bytes ({:.2} MB)",
            size,
            size as f64 / (1024.0 * 1024.0)
        );
        println!("  Entries: {}", count);
        println!(
            "  Average entry size: {} bytes",
            if count > 0 { size / count as u64 } else { 0 }
        );
    }
    else {
        println!("WAL Statistics for all collections:");

        let collections = store.list_collections().await?;
        let mut total_size = 0u64;
        let mut total_entries = 0usize;

        for collection_name in collections {
            if let Ok(collection) = store.collection(&collection_name).await {
                if let (Ok(size), Ok(count)) = (
                    collection.wal_size().await,
                    collection.wal_entries_count().await,
                ) {
                    total_size += size;
                    total_entries += count;

                    println!("  {}: {} bytes, {} entries", collection_name, size, count);
                }
            }
        }

        println!(
            "  Total: {} bytes ({:.2} MB), {} entries",
            total_size,
            total_size as f64 / (1024.0 * 1024.0),
            total_entries
        );
    }

    Ok(())
}
