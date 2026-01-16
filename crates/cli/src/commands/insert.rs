use clap::Args;
use serde_json::Value;
use tracing::{error, info};

/// Arguments for the insert command.
#[derive(Args, Clone, Default)]
pub struct InsertArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path: String,
    /// Collection name
    #[arg(short, long)]
    pub collection: String,
    /// Document ID (not used with --bulk)
    #[arg(short, long)]
    pub id:         Option<String>,
    /// JSON data (as string, not used with --bulk)
    #[arg(short, long)]
    pub data:       Option<String>,
    /// Bulk insert from JSON file (format: {"id1": {...}, "id2": {...}})
    #[arg(short, long)]
    pub bulk:       Option<String>,
    /// Passphrase for decrypting the signing key
    #[arg(long)]
    pub passphrase: Option<String>,
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
/// * `args` - The parsed command-line arguments for insert.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::insert::{run, InsertArgs};
///
/// // Single document insert
/// let args = InsertArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     id:         Some("user1".to_string()),
///     data:       Some(r#"{"name": "Alice"}"#.to_string()),
///     bulk:       None,
///     passphrase: None,
/// };
/// run(args).await?;
///
/// // Bulk insert
/// let args = InsertArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     id:         None,
///     data:       None,
///     bulk:       Some("documents.json".to_string()),
///     passphrase: None,
/// };
/// run(args).await?;
/// ```
pub async fn run(args: InsertArgs) -> sentinel_dbms::Result<()> {
    let store = sentinel_dbms::Store::new(&args.store_path, args.passphrase.as_deref()).await?;
    let coll = store.collection(&args.collection).await?;

    if let Some(bulk_file) = args.bulk {
        insert_bulk_documents(coll, &args.store_path, &args.collection, bulk_file).await
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
        insert_single_document(coll, &args.store_path, &args.collection, &id, &data).await
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

/// Insert multiple documents from a JSON file into the collection.
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
        "Bulk inserting documents into collection '{}' in store {} from file '{}'",
        collection, store_path, bulk_file
    );

    let json_str = read_bulk_file(&bulk_file)?;
    let documents = parse_json_string(&json_str)?;

    let obj = validate_bulk_json_object(&documents, &bulk_file)?;

    let docs_to_insert = prepare_bulk_documents(obj);

    info!("Inserting {} documents", docs_to_insert.len());

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

/// Read and return the contents of a bulk insert JSON file.
///
/// # Arguments
/// * `file_path` - Path to the JSON file
///
/// # Returns
/// Returns the file contents as a string, or a `SentinelError` on failure.
fn read_bulk_file(file_path: &str) -> Result<String, sentinel_dbms::SentinelError> {
    std::fs::read_to_string(file_path).map_err(|e| {
        error!("Failed to read file '{}': {}", file_path, e);
        sentinel_dbms::SentinelError::Io {
            source: e,
        }
    })
}

/// Parse a JSON string into a serde_json::Value.
///
/// # Arguments
/// * `json_str` - The JSON string to parse
///
/// # Returns
/// Returns the parsed JSON value, or a `SentinelError` on failure.
fn parse_json_string(json_str: &str) -> Result<Value, sentinel_dbms::SentinelError> {
    serde_json::from_str(json_str).map_err(|e| {
        error!("Invalid JSON data: {}", e);
        sentinel_dbms::SentinelError::Json {
            source: e,
        }
    })
}

/// Validate that the parsed JSON is an object suitable for bulk insert.
///
/// # Arguments
/// * `documents` - The parsed JSON value
/// * `bulk_file` - File path for error messages
///
/// # Returns
/// Returns the JSON object, or a `SentinelError` on failure.
fn validate_bulk_json_object<'a>(
    documents: &'a Value,
    bulk_file: &str,
) -> Result<&'a serde_json::Map<String, Value>, sentinel_dbms::SentinelError> {
    documents.as_object().ok_or_else(|| {
        error!(
            "JSON data in file '{}' must be an object with document IDs as keys",
            bulk_file
        );
        sentinel_dbms::SentinelError::Internal {
            message: "Expected JSON object".to_owned(),
        }
    })
}

/// Prepare documents for bulk insert from a JSON object.
///
/// # Arguments
/// * `obj` - The JSON object containing document ID -> data mappings
///
/// # Returns
/// Returns a vector of (id, data) tuples ready for bulk insert.
fn prepare_bulk_documents(obj: &serde_json::Map<String, Value>) -> Vec<(&str, Value)> {
    obj.iter()
        .map(|(id, data)| (id.as_str(), data.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    /// Test successful document insertion.
    ///
    /// This test verifies that insert succeeds with valid JSON data.
    /// It sets up a store and collection, then inserts a document.
    #[tokio::test]
    async fn test_insert_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store and collection
        let init_args = crate::commands::init::InitArgs {
            path:        store_path.to_string_lossy().to_string(),
            passphrase:  None,
            signing_key: None,
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         Some("doc1".to_string()),
            data:       Some(r#"{"name": "Alice", "age": 30}"#.to_string()),
            bulk:       None,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Insert should succeed with valid data");
    }

    /// Test insert with invalid JSON.
    ///
    /// This test checks that insert fails and returns appropriate error
    /// when provided with malformed JSON data.
    #[tokio::test]
    async fn test_insert_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store and collection
        let init_args = crate::commands::init::InitArgs {
            path:        store_path.to_string_lossy().to_string(),
            passphrase:  None,
            signing_key: None,
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         Some("doc1".to_string()),
            data:       Some(r#"{"name": "Alice", "age": }"#.to_string()), // Invalid JSON
            bulk:       None,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_err(), "Insert should fail with invalid JSON");
    }

    /// Test insert without document ID.
    ///
    /// This test checks that insert fails when no ID is provided.
    #[tokio::test]
    async fn test_insert_missing_id() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store and collection
        let init_args = crate::commands::init::InitArgs {
            path:        store_path.to_string_lossy().to_string(),
            passphrase:  None,
            signing_key: None,
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         None, // Missing ID
            data:       Some(r#"{"name": "Alice"}"#.to_string()),
            bulk:       None,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_err(), "Insert should fail without ID");
    }

    /// Test insert without document data.
    ///
    /// This test checks that insert fails when no data is provided.
    #[tokio::test]
    async fn test_insert_missing_data() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store and collection
        let init_args = crate::commands::init::InitArgs {
            path:        store_path.to_string_lossy().to_string(),
            passphrase:  None,
            signing_key: None,
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         Some("doc1".to_string()),
            data:       None, // Missing data
            bulk:       None,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_err(), "Insert should fail without data");
    }

    /// Test insert into non-existent collection.
    ///
    /// This test verifies that insert creates the collection if it does not exist.
    #[tokio::test]
    async fn test_insert_non_existent_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store only
        let init_args = crate::commands::init::InitArgs {
            path:        store_path.to_string_lossy().to_string(),
            passphrase:  None,
            signing_key: None,
        };
        crate::commands::init::run(init_args).await.unwrap();

        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "non_existent".to_string(),
            id:         Some("doc1".to_string()),
            data:       Some(r#"{"name": "Alice"}"#.to_string()),
            bulk:       None,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Insert should create collection if needed");
    }

    /// Test insert with empty data.
    ///
    /// This test checks behavior with empty JSON object.
    #[tokio::test]
    async fn test_insert_empty_data() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store and collection
        let init_args = crate::commands::init::InitArgs {
            path:        store_path.to_string_lossy().to_string(),
            passphrase:  None,
            signing_key: None,
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         Some("doc1".to_string()),
            data:       Some("{}".to_string()),
            bulk:       None,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(
            result.is_ok(),
            "Insert should succeed with empty JSON object"
        );
    }

    /// Test insert with complex JSON.
    ///
    /// This test verifies that complex JSON structures (arrays, nested objects)
    /// are handled correctly.
    #[tokio::test]
    async fn test_insert_complex_json() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store and collection
        let init_args = crate::commands::init::InitArgs {
            path:        store_path.to_string_lossy().to_string(),
            passphrase:  None,
            signing_key: None,
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         Some("doc1".to_string()),
            data:       Some(
                r#"{"users": [{"name": "Alice"}, {"name": "Bob"}], "metadata": {"version": 1}}"#.to_string(),
            ),
            bulk:       None,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Insert should succeed with complex JSON");
    }

    /// Test insert with read-only collection.
    ///
    /// This test verifies that insert fails when the collection directory
    /// is read-only, covering the error branch.
    #[tokio::test]
    async fn test_insert_readonly_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init store and create collection
        let init_args = crate::commands::init::InitArgs {
            path:        store_path.to_string_lossy().to_string(),
            passphrase:  None,
            signing_key: None,
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
            ..Default::default()
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        // Make the collection directory read-only
        let collection_path = store_path.join("data").join("test_collection");
        let mut perms = std::fs::metadata(&collection_path).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&collection_path, perms).unwrap();

        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         Some("doc1".to_string()),
            data:       Some(r#"{"name": "Alice"}"#.to_string()),
            bulk:       None,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(
            result.is_err(),
            "Insert should fail on read-only collection"
        );

        // Restore permissions for cleanup
        let mut perms = std::fs::metadata(&collection_path).unwrap().permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(&collection_path, perms).unwrap();
    }

    /// Test bulk insert via CLI.
    ///
    /// This test verifies that bulk insert works through the CLI interface.
    #[tokio::test]
    async fn test_bulk_insert_via_cli() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let json_file = temp_dir.path().join("bulk_data.json");

        // Create test JSON file
        let test_data = json!({
            "user1": {"name": "Alice", "role": "admin"},
            "user2": {"name": "Bob", "role": "user"},
            "user3": {"name": "Charlie", "role": "user"}
        });
        std::fs::write(
            &json_file,
            serde_json::to_string_pretty(&test_data).unwrap(),
        )
        .unwrap();

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Test bulk insert via CLI
        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         None,
            data:       None,
            bulk:       Some(json_file.to_string_lossy().to_string()),
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Bulk insert should succeed");

        // Verify documents were inserted
        let ids = collection.list().await.unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"user1".to_string()));
        assert!(ids.contains(&"user2".to_string()));
        assert!(ids.contains(&"user3".to_string()));
    }
}
