//! WAL list command.

use clap::Args;
use serde_json::json;
use sentinel_dbms::futures::StreamExt;
use tracing::info;

/// Arguments for the WAL list command.
#[derive(Args)]
pub struct ListArgs {
    /// Maximum number of entries to display
    #[arg(short, long, default_value = "50")]
    pub limit: usize,

    /// Output format (json, table)
    #[arg(short, long, default_value = "table")]
    pub format: String,
}

/// Execute the WAL list operation.
pub async fn run(store_path: String, collection: Option<String>, args: ListArgs) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::{CollectionWalOps, StoreWalOps};

    let store =
        sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default()).await?;

    if let Some(collection_name) = collection {
        let collection = store.collection_with_config(&collection_name, None).await?;
        info!(
            "Listing WAL entries for collection '{}' (limit: {})...",
            collection_name, args.limit
        );

        let mut stream = collection.stream_wal_entries().await?;
        let mut count = 0;

        while let Some(result) = stream.next().await {
            if count >= args.limit {
                println!("... (truncated, showing first {} entries)", args.limit);
                break;
            }

            let entry = result?;
            count += 1;

            match args.format.as_str() {
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
                        message: format!("Unsupported format: {}", args.format),
                    });
                },
            }
        }

        info!("Total entries shown: {}", count);
    }
    else {
        info!(
            "Listing WAL entries for all collections (limit: {})...",
            args.limit
        );

        let mut stream = store.stream_all_wal_entries().await?;
        let mut count = 0;

        while let Some(result) = stream.next().await {
            if count >= args.limit {
                println!("... (truncated, showing first {} entries)", args.limit);
                break;
            }

            let (collection_name, entry) = result?;
            count += 1;

            match args.format.as_str() {
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
                        message: format!("Unsupported format: {}", args.format),
                    });
                },
            }
        }

        info!("Total entries shown: {}", count);
    }

    Ok(())
}
