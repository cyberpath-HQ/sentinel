use std::fs;

use clap::Args;
use serde_json::Value;
use tracing::info;

/// Arguments for collection bulk-insert command.
#[derive(Args)]
pub struct BulkInsertArgs {
    /// JSON file containing array of documents to insert
    /// Format: [{"id": "doc1", "data": {...}}, {"id": "doc2", "data": {...}}]
    #[arg(short, long)]
    pub file: String,

    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal: crate::commands::WalArgs,
}

/// Execute collection bulk-insert command.
///
/// Inserts multiple documents into the specified collection from a JSON file.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection_name` - Name of the collection
/// * `passphrase` - Optional passphrase for decrypting signing key
/// * `args` - Bulk-insert command arguments
///
/// # Returns
/// Returns `Ok(())` on success.
pub async fn run(
    store_path: String,
    collection_name: String,
    passphrase: Option<String>,
    args: BulkInsertArgs,
) -> sentinel_dbms::Result<()> {
    // Read and parse the JSON file
    let content = fs::read_to_string(&args.file).map_err(|e| {
        sentinel_dbms::SentinelError::Io {
            source: e,
        }
    })?;

    let json_value: Value = serde_json::from_str(&content).map_err(|e| {
        sentinel_dbms::SentinelError::Json {
            source: e,
        }
    })?;

    let documents_array = json_value.as_array().ok_or_else(|| {
        sentinel_dbms::SentinelError::Internal {
            message: "JSON file must contain an array of documents".to_string(),
        }
    })?;

    if documents_array.is_empty() {
        info!("No documents to insert");
        return Ok(());
    }

    // Parse documents into (id, data) pairs
    let mut docs_to_insert = Vec::new();
    for doc_value in documents_array {
        let doc_obj = doc_value.as_object().ok_or_else(|| {
            sentinel_dbms::SentinelError::Internal {
                message: "Each document must be an object".to_string(),
            }
        })?;

        let id = doc_obj.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
            sentinel_dbms::SentinelError::Internal {
                message: "Each document must have an 'id' field with a string value".to_string(),
            }
        })?;

        let data = doc_obj.get("data").ok_or_else(|| {
            sentinel_dbms::SentinelError::Internal {
                message: "Each document must have a 'data' field".to_string(),
            }
        })?;

        docs_to_insert.push((id, data.clone()));
    }

    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let collection = store
        .collection_with_config(&collection_name, Some(args.wal.to_overrides()))
        .await?;

    let doc_count = docs_to_insert.len();

    collection.bulk_insert(docs_to_insert).await?;

    info!(
        "Successfully inserted {} documents into collection '{}'",
        doc_count, collection_name
    );

    Ok(())
}
