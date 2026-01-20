use clap::Args;
#[cfg(test)]
use sentinel_dbms::futures::TryStreamExt;
use serde_json::Value;
use tracing::{error, info};

use crate::commands::WalArgs;

/// Arguments for the collection insert command.
#[derive(Args, Clone, Default)]
pub struct InsertArgs {
    /// Document ID (not used with --bulk)
    #[arg(short, long)]
    pub id:   Option<String>,
    /// JSON data (as string, not used with --bulk)
    #[arg(short, long)]
    pub data: Option<String>,
    /// Bulk insert from JSON file (format: {"id1": {...}, "id2": {...}})
    #[arg(short, long)]
    pub bulk: Option<String>,
    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal:  WalArgs,
}

/// Insert a new document into a Sentinel collection.
///
/// This function can operate in two modes:
/// 1. Single document insert: Provide --id and --data
/// 2. Bulk insert: Provide --bulk with a JSON file containing multiple documents
///
/// For bulk insert, the JSON file should be an object where keys are
/// document IDs and values are the document data.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection` - Collection name
/// * `passphrase` - Optional passphrase for decrypting the signing key
/// * `args` - The parsed command-line arguments for collection insert.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::collection::insert::{run, InsertArgs};
///
/// // Single document insert
/// let args = InsertArgs {
///     id:   Some(String::from("user1")),
///     data: Some(r#"{"name": "Alice"}"#.to_string()),
///     bulk: None,
///     wal:  WalArgs::default(),
/// };
/// run(
///     String::from("/tmp/my_store"),
///     String::from("users"),
///     None,
///     args,
/// )
/// .await?;
/// ```

/// Parse a JSON string into a serde_json::Value
fn parse_json_string(json_str: &str) -> sentinel_dbms::Result<Value> {
    serde_json::from_str(json_str).map_err(|e| {
        sentinel_dbms::SentinelError::Json {
            source: e,
        }
    })
}

pub async fn run(
    store_path: String,
    collection: String,
    passphrase: Option<String>,
    args: InsertArgs,
) -> sentinel_dbms::Result<()> {
    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let coll = store
        .collection_with_config(&collection, Some(args.wal.to_overrides()))
        .await?;

    if let Some(bulk_file) = args.bulk {
        insert_bulk_documents(coll, &store_path, &collection, bulk_file).await
    }
    else {
        let id = args.id.ok_or_else(|| {
            sentinel_dbms::SentinelError::Internal {
                message: "Document ID is required for single insert mode".to_owned(),
            }
        })?;
        let data = args.data.ok_or_else(|| {
            sentinel_dbms::SentinelError::Internal {
                message: "Document data is required for single insert mode".to_owned(),
            }
        })?;
        insert_single_document(coll, &store_path, &collection, &id, &data).await
    }
}

/// Insert a single document into the collection.
///
/// # Arguments
/// * `coll` - The collection to insert into
/// * `store_path` - Path to the store for logging
/// * `collection` - Collection name for logging
/// * `id` - Document ID
/// * `data` - JSON data as string
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
async fn insert_single_document(
    coll: sentinel_dbms::Collection,
    store_path: &str,
    collection: &str,
    id: &str,
    data: &str,
) -> sentinel_dbms::Result<()> {
    info!(
        "Inserting document '{}' into collection '{}' in store {}",
        id, collection, store_path
    );

    let value = parse_json_string(data)?;

    match coll.insert(id, value).await {
        Ok(_) => {
            info!("Document '{}' inserted successfully", id);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to insert document '{}' into collection '{}' in store {}: {}",
                id, collection, store_path, e
            );
            Err(e)
        },
    }
}

/// Insert multiple documents from a JSON file.
///
/// The JSON file should contain an object where keys are document IDs
/// and values are the document data.
///
/// # Arguments
/// * `coll` - The collection to insert into
/// * `store_path` - Path to the store for logging
/// * `collection` - Collection name for logging
/// * `bulk_file` - Path to the JSON file containing documents
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
async fn insert_bulk_documents(
    coll: sentinel_dbms::Collection,
    store_path: &str,
    collection: &str,
    bulk_file: String,
) -> sentinel_dbms::Result<()> {
    info!(
        "Bulk inserting documents from '{}' into collection '{}' in store {}",
        bulk_file, collection, store_path
    );

    let content = tokio::fs::read_to_string(&bulk_file).await.map_err(|e| {
        sentinel_dbms::SentinelError::Io {
            source: e,
        }
    })?;

    let documents: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_str(&content).map_err(|e| {
            sentinel_dbms::SentinelError::Json {
                source: e,
            }
        })?;

    let mut success_count = 0;
    let mut error_count = 0;

    for (id, data) in documents {
        match coll.insert(&id, data).await {
            Ok(_) => {
                success_count += 1;
                info!("Document '{}' inserted successfully", id);
            },
            Err(e) => {
                error_count += 1;
                error!("Failed to insert document '{}': {}", id, e);
            },
        }
    }

    info!(
        "Bulk insert completed: {} successful, {} failed",
        success_count, error_count
    );

    if error_count > 0 {
        return Err(sentinel_dbms::SentinelError::Internal {
            message: format!("Bulk insert had {} failures", error_count),
        });
    }

    Ok(())
}
