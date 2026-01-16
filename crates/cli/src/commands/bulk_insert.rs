use std::fs;

use clap::Args;
use serde_json::Value;
use tracing::{error, info};

/// Arguments for the bulk-insert command.
#[derive(Args, Clone, Default)]
pub struct BulkInsertArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path: String,
    /// Collection name
    #[arg(short, long)]
    pub collection: String,
    /// JSON file containing documents to insert (format: {"id1": {...}, "id2": {...}})
    #[arg(short, long)]
    pub file:       Option<String>,
    /// Passphrase for decrypting the signing key
    #[arg(long)]
    pub passphrase: Option<String>,
}

/// Bulk insert multiple documents into a Sentinel collection.
///
/// This function reads a JSON file containing multiple documents and inserts them
/// into the specified collection. The JSON file should be an object where keys are
/// document IDs and values are the document data.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for bulk-insert.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::bulk_insert::{run, BulkInsertArgs};
///
/// let args = BulkInsertArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     file:       Some("documents.json".to_string()),
///     passphrase: None,
/// };
/// run(args).await?;
/// ```
pub async fn run(args: BulkInsertArgs) -> sentinel_dbms::Result<()> {
    let store_path = args.store_path;
    let collection = args.collection;
    info!(
        "Bulk inserting documents into collection '{}' in store {}",
        collection, store_path
    );

    // Read JSON data from file or stdin
    let json_str = if let Some(file_path) = args.file {
        match fs::read_to_string(&file_path) {
            Ok(content) => content,
            Err(e) => {
                error!("Failed to read file '{}': {}", file_path, e);
                return Err(sentinel_dbms::SentinelError::Io {
                    source: e,
                });
            },
        }
    }
    else {
        // Read from stdin
        use std::io::{self, Read};
        let mut buffer = String::new();
        match io::stdin().read_to_string(&mut buffer) {
            Ok(_) => buffer,
            Err(e) => {
                error!("Failed to read from stdin: {}", e);
                return Err(sentinel_dbms::SentinelError::Io {
                    source: e,
                });
            },
        }
    };

    // Parse JSON
    let documents: Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => {
            error!("Invalid JSON data: {}", e);
            return Err(sentinel_dbms::SentinelError::Json {
                source: e,
            });
        },
    };

    // Validate that it's an object
    let obj = match documents.as_object() {
        Some(o) => o,
        None => {
            error!("JSON data must be an object with document IDs as keys");
            return Err(sentinel_dbms::SentinelError::Internal {
                message: "Expected JSON object".to_string(),
            });
        },
    };

    // Prepare documents for bulk insert
    let mut docs_to_insert = Vec::new();
    for (id, data) in obj {
        docs_to_insert.push((id.as_str(), data.clone()));
    }

    info!("Inserting {} documents", docs_to_insert.len());

    // Perform bulk insert
    let store = sentinel_dbms::Store::new(&store_path, args.passphrase.as_deref()).await?;
    let coll = store.collection(&collection).await?;
    match coll.bulk_insert(docs_to_insert).await {
        Ok(_) => {
            info!(
                "Successfully inserted {} documents into collection '{}'",
                obj.len(),
                collection
            );
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to bulk insert documents into collection '{}' in store {}: {}",
                collection, store_path, e
            );
            Err(e)
        },
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    /// Test successful bulk insert.
    ///
    /// This test creates a JSON file with multiple documents and bulk inserts them.
    #[tokio::test]
    async fn test_bulk_insert_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let json_file = temp_dir.path().join("docs.json");

        // Create test JSON file
        let test_data = json!({
            "doc1": {"name": "Alice", "age": 30},
            "doc2": {"name": "Bob", "age": 25},
            "doc3": {"name": "Charlie", "age": 35}
        });
        fs::write(
            &json_file,
            serde_json::to_string_pretty(&test_data).unwrap(),
        )
        .unwrap();

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Test bulk insert command
        let args = BulkInsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            file:       Some(json_file.to_string_lossy().to_string()),
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok());

        // Verify documents were inserted
        let ids = collection.list().await.unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"doc1".to_string()));
        assert!(ids.contains(&"doc2".to_string()));
        assert!(ids.contains(&"doc3".to_string()));
    }

    /// Test bulk insert with invalid JSON.
    ///
    /// This test verifies that invalid JSON is properly rejected.
    #[tokio::test]
    async fn test_bulk_insert_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let json_file = temp_dir.path().join("invalid.json");

        // Create invalid JSON file
        fs::write(&json_file, "invalid json content").unwrap();

        // Setup store
        let _store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();

        // Test bulk insert command
        let args = BulkInsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            file:       Some(json_file.to_string_lossy().to_string()),
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_err());
    }

    /// Test bulk insert with non-object JSON.
    ///
    /// This test verifies that non-object JSON (like an array) is rejected.
    #[tokio::test]
    async fn test_bulk_insert_non_object_json() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let json_file = temp_dir.path().join("array.json");

        // Create array JSON file
        let test_data = json!([
            {"id": "doc1", "data": {"name": "Alice"}},
            {"id": "doc2", "data": {"name": "Bob"}}
        ]);
        fs::write(
            &json_file,
            serde_json::to_string_pretty(&test_data).unwrap(),
        )
        .unwrap();

        // Setup store
        let _store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();

        // Test bulk insert command
        let args = BulkInsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            file:       Some(json_file.to_string_lossy().to_string()),
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_err());
    }
}
