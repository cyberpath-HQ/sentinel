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
            message: String::from("JSON file must contain an array of documents"),
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
                message: String::from("Each document must be an object"),
            }
        })?;

        let id = doc_obj.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
            sentinel_dbms::SentinelError::Internal {
                message: String::from("Each document must have an 'id' field with a string value"),
            }
        })?;

        let data = doc_obj.get("data").ok_or_else(|| {
            sentinel_dbms::SentinelError::Internal {
                message: String::from("Each document must have a 'data' field"),
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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_bulk_insert_valid_json() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        // Create collection
        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        // Create JSON file with documents
        let json_content = r#"[
            {"id": "doc1", "data": {"name": "Alice", "age": 30}},
            {"id": "doc2", "data": {"name": "Bob", "age": 25}}
        ]"#;
        let json_file = temp_dir.path().join("docs.json");
        fs::write(&json_file, json_content).unwrap();

        let args = BulkInsertArgs {
            file: json_file.to_string_lossy().to_string(),
            wal:  crate::commands::WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_ok());

        // Verify documents were inserted
        let collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();
        let doc1_opt = collection.get("doc1").await.unwrap();
        assert!(doc1_opt.is_some());
        let doc1 = doc1_opt.unwrap();
        let doc1_data = doc1.data();
        assert_eq!(doc1_data["name"], "Alice");
        assert_eq!(doc1_data["age"], 30);

        let doc2_opt = collection.get("doc2").await.unwrap();
        assert!(doc2_opt.is_some());
        let doc2 = doc2_opt.unwrap();
        let doc2_data = doc2.data();
        assert_eq!(doc2_data["name"], "Bob");
        assert_eq!(doc2_data["age"], 25);
    }

    #[tokio::test]
    async fn test_bulk_insert_empty_array() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        // Create collection
        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        // Create JSON file with empty array
        let json_content = "[]";
        let json_file = temp_dir.path().join("empty.json");
        fs::write(&json_file, json_content).unwrap();

        let args = BulkInsertArgs {
            file: json_file.to_string_lossy().to_string(),
            wal:  crate::commands::WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bulk_insert_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        // Create collection
        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        // Create invalid JSON file
        let json_content = r#"{"invalid": "json"}"#;
        let json_file = temp_dir.path().join("invalid.json");
        fs::write(&json_file, json_content).unwrap();

        let args = BulkInsertArgs {
            file: json_file.to_string_lossy().to_string(),
            wal:  crate::commands::WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bulk_insert_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        // Create collection
        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let args = BulkInsertArgs {
            file: "nonexistent.json".to_string(),
            wal:  crate::commands::WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bulk_insert_invalid_document_structure() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";

        // Initialize store
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        // Create collection
        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        // Create JSON file with invalid document structure
        let json_content = r#"[
            {"id": "doc1", "data": {"name": "Alice"}},
            {"invalid": "structure"}
        ]"#;
        let json_file = temp_dir.path().join("invalid_docs.json");
        fs::write(&json_file, json_content).unwrap();

        let args = BulkInsertArgs {
            file: json_file.to_string_lossy().to_string(),
            wal:  crate::commands::WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_err());
    }
}
