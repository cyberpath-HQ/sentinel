//! WAL list command.

use clap::Args;
use serde_json::json;
use sentinel_dbms::futures::StreamExt as _;
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
#[allow(clippy::unreachable, clippy::print_stdout, clippy::arithmetic_side_effects, reason = "CLI command output and safe counting")]
pub async fn run(store_path: String, collection: Option<String>, args: ListArgs) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::{CollectionWalOps as _, StoreWalOps as _};

    // Validate format before starting
    match args.format.as_str() {
        "json" | "table" => {},
        _ => {
            return Err(sentinel_dbms::SentinelError::ConfigError {
                message: format!("Unsupported format: {}", args.format),
            });
        },
    }

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
                            .unwrap_or_else(|| "invalid timestamp".to_owned())
                    );
                },
                _ => unreachable!("Format should have been validated at function start"),
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
                            .unwrap_or_else(|| String::from("invalid timestamp"))
                    );
                },
                _ => unreachable!("Format should have been validated at function start"),
            }
        }

        info!("Total entries shown: {}", count);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use sentinel_dbms::StoreWalConfig;
    use tempfile::TempDir;
    use tokio::time::{sleep, Duration};

    use super::*;

    #[tokio::test]
    async fn test_list_wal_entries_specific_collection_table_format() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, StoreWalConfig::default())
            .await
            .unwrap();

        let collection = store
            .collection_with_config("test_collection", None)
            .await
            .unwrap();

        // Insert some data to create WAL entries
        collection
            .insert("doc1", serde_json::json!({"name": "test1"}))
            .await
            .unwrap();
        collection
            .insert("doc2", serde_json::json!({"name": "test2"}))
            .await
            .unwrap();

        // Wait for events to be processed
        sleep(Duration::from_millis(100)).await;

        // Run list command
        let args = ListArgs {
            limit:  10,
            format: "table".to_string(),
        };
        let result = run(
            store_path.clone(),
            Some("test_collection".to_string()),
            args,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_wal_entries_all_collections_json_format() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create store and collections
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, StoreWalConfig::default())
            .await
            .unwrap();

        let collection1 = store
            .collection_with_config("collection1", None)
            .await
            .unwrap();
        let collection2 = store
            .collection_with_config("collection2", None)
            .await
            .unwrap();

        // Insert some data to create WAL entries
        collection1
            .insert("doc1", serde_json::json!({"name": "test1"}))
            .await
            .unwrap();
        collection2
            .insert("doc2", serde_json::json!({"name": "test2"}))
            .await
            .unwrap();

        // Wait for events to be processed
        sleep(Duration::from_millis(100)).await;

        // Run list command for all collections
        let args = ListArgs {
            limit:  5,
            format: "json".to_string(),
        };
        let result = run(store_path.clone(), None, args).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_wal_entries_empty_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, StoreWalConfig::default())
            .await
            .unwrap();

        let _collection = store
            .collection_with_config("empty_collection", None)
            .await
            .unwrap();

        // Run list command on empty collection
        let args = ListArgs {
            limit:  10,
            format: "table".to_string(),
        };
        let result = run(
            store_path.clone(),
            Some("empty_collection".to_string()),
            args,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_wal_entries_unsupported_format() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, StoreWalConfig::default())
            .await
            .unwrap();

        let _collection = store
            .collection_with_config("test_collection", None)
            .await
            .unwrap();

        // Run list command with unsupported format
        let args = ListArgs {
            limit:  10,
            format: "xml".to_string(),
        };
        let result = run(
            store_path.clone(),
            Some("test_collection".to_string()),
            args,
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            sentinel_dbms::SentinelError::ConfigError {
                message,
            } => {
                assert!(message.contains("Unsupported format"));
            },
            _ => panic!("Expected ConfigError"),
        }
    }

    #[tokio::test]
    async fn test_list_wal_entries_all_collections_unsupported_format() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create store
        sentinel_dbms::Store::new_with_config(&store_path, None, StoreWalConfig::default())
            .await
            .unwrap();

        // Run list command with unsupported format for all collections
        let args = ListArgs {
            limit:  10,
            format: "xml".to_string(),
        };
        let result = run(store_path.clone(), None, args).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            sentinel_dbms::SentinelError::ConfigError {
                message,
            } => {
                assert!(message.contains("Unsupported format"));
            },
            _ => panic!("Expected ConfigError"),
        }
    }
}
