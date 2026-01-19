use clap::Args;
#[cfg(test)]
use sentinel_dbms::futures::TryStreamExt;
use serde_json::Value;
use tracing::{error, info};
use sentinel_dbms::{CollectionWalConfig, CompressionAlgorithm, WalFailureMode, WalFormat};

use crate::commands::WalArgs;

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
    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal:        WalArgs,
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
/// use sentinel_cli::commands::{
///     insert::{run, InsertArgs},
///     WalArgs,
/// };
///
/// // Single document insert
/// let args = InsertArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     id:         Some("user1".to_string()),
///     data:       Some(r#"{"name": "Alice"}"#.to_string()),
///     bulk:       None,
///     passphrase: None,
///     wal:        WalArgs::default(),
/// };
/// run(args, &WalArgs::default()).await?;
///
/// // Bulk insert
/// let args = InsertArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     id:         None,
///     data:       None,
///     bulk:       Some("documents.json".to_string()),
///     passphrase: None,
///     wal:        WalArgs::default(),
/// };
/// run(args, &WalArgs::default()).await?;
/// ```

/// Build CollectionWalConfig from CLI arguments
fn build_collection_wal_config(args: &InsertArgs, global_wal: &WalArgs) -> Option<CollectionWalConfig> {
    // Only build config if any WAL options are provided
    if args.wal.wal_max_file_size.is_some() ||
        args.wal.wal_format.is_some() ||
        args.wal.wal_compression.is_some() ||
        args.wal.wal_max_records.is_some() ||
        args.wal.wal_write_mode.is_some() ||
        args.wal.wal_verify_mode.is_some() ||
        args.wal.wal_auto_verify.is_some() ||
        args.wal.wal_enable_recovery.is_some() ||
        global_wal.wal_max_file_size.is_some() ||
        global_wal.wal_format.is_some() ||
        global_wal.wal_compression.is_some() ||
        global_wal.wal_max_records.is_some() ||
        global_wal.wal_write_mode.is_some() ||
        global_wal.wal_verify_mode.is_some() ||
        global_wal.wal_auto_verify.is_some() ||
        global_wal.wal_enable_recovery.is_some()
    {
        Some(CollectionWalConfig {
            write_mode:            args
                .wal
                .wal_write_mode
                .or(global_wal.wal_write_mode)
                .unwrap_or(WalFailureMode::Strict),
            verification_mode:     args
                .wal
                .wal_verify_mode
                .or(global_wal.wal_verify_mode)
                .unwrap_or(WalFailureMode::Warn),
            auto_verify:           args
                .wal
                .wal_auto_verify
                .or(global_wal.wal_auto_verify)
                .unwrap_or(false),
            enable_recovery:       args
                .wal
                .wal_enable_recovery
                .or(global_wal.wal_enable_recovery)
                .unwrap_or(true),
            max_wal_size_bytes:    args.wal.wal_max_file_size.or(global_wal.wal_max_file_size),
            compression_algorithm: args.wal.wal_compression.or(global_wal.wal_compression),
            max_records_per_file:  args.wal.wal_max_records.or(global_wal.wal_max_records),
            format:                args
                .wal
                .wal_format
                .or(global_wal.wal_format)
                .unwrap_or_default(),
        })
    }
    else {
        None
    }
}

pub async fn run(args: InsertArgs, global_wal: &WalArgs) -> sentinel_dbms::Result<()> {
    let store = sentinel_dbms::Store::new_with_config(
        &args.store_path,
        args.passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let wal_config = build_collection_wal_config(&args, global_wal);
    let coll = store
        .collection_with_config(&args.collection, wal_config)
        .await?;

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
            path: store_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            ..Default::default()
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
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
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
            path: store_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            ..Default::default()
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
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
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
            path: store_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            ..Default::default()
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
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
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
            path: store_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            ..Default::default()
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
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
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
            path: store_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            ..Default::default()
        };
        crate::commands::init::run(init_args).await.unwrap();

        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "non_existent".to_string(),
            id:         Some("doc1".to_string()),
            data:       Some(r#"{"name": "Alice"}"#.to_string()),
            bulk:       None,
            passphrase: None,
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
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
            path: store_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            ..Default::default()
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
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
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
            path: store_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            ..Default::default()
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
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
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
            path: store_path.to_string_lossy().to_string(),
            passphrase: None,
            signing_key: None,
            ..Default::default()
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
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
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
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        let collection = store
            .collection_with_config("test_collection", None)
            .await
            .unwrap();

        // Test bulk insert via CLI
        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         None,
            data:       None,
            bulk:       Some(json_file.to_string_lossy().to_string()),
            passphrase: None,
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
        assert!(result.is_ok(), "Bulk insert should succeed");

        // Verify documents were inserted
        let ids: Vec<String> = collection.list().try_collect().await.unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"user1".to_string()));
        assert!(ids.contains(&"user2".to_string()));
        assert!(ids.contains(&"user3".to_string()));
    }

    /// Test bulk insert with invalid JSON file path.
    #[tokio::test]
    async fn test_bulk_insert_invalid_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        store
            .collection_with_config("test_collection", None)
            .await
            .unwrap();

        // Test bulk insert with non-existent file
        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         None,
            data:       None,
            bulk:       Some("non_existent_file.json".to_string()),
            passphrase: None,
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
        assert!(
            result.is_err(),
            "Bulk insert should fail with invalid file path"
        );
    }

    /// Test bulk insert with invalid JSON content.
    #[tokio::test]
    async fn test_bulk_insert_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let json_file = temp_dir.path().join("invalid.json");

        // Create invalid JSON file
        std::fs::write(&json_file, "invalid json content").unwrap();

        // Setup store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        store
            .collection_with_config("test_collection", None)
            .await
            .unwrap();

        // Test bulk insert with invalid JSON
        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         None,
            data:       None,
            bulk:       Some(json_file.to_string_lossy().to_string()),
            passphrase: None,
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
        assert!(result.is_err(), "Bulk insert should fail with invalid JSON");
    }

    /// Test bulk insert with JSON that is not an object.
    #[tokio::test]
    async fn test_bulk_insert_json_not_object() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let json_file = temp_dir.path().join("array.json");

        // Create JSON array file (not object)
        std::fs::write(&json_file, "[1, 2, 3]").unwrap();

        // Setup store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        store
            .collection_with_config("test_collection", None)
            .await
            .unwrap();

        // Test bulk insert with JSON array
        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         None,
            data:       None,
            bulk:       Some(json_file.to_string_lossy().to_string()),
            passphrase: None,
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
        assert!(result.is_err(), "Bulk insert should fail with JSON array");
    }

    /// Test bulk insert into non-existent collection.
    #[tokio::test]
    async fn test_bulk_insert_non_existent_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let json_file = temp_dir.path().join("data.json");

        // Create valid JSON file
        let test_data = json!({"user1": {"name": "Alice"}});
        std::fs::write(
            &json_file,
            serde_json::to_string_pretty(&test_data).unwrap(),
        )
        .unwrap();

        // Setup store but not collection
        sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        // Test bulk insert into non-existent collection (should succeed - collection gets created)
        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "non_existent_collection".to_string(),
            id:         None,
            data:       None,
            bulk:       Some(json_file.to_string_lossy().to_string()),
            passphrase: None,
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
        assert!(
            result.is_ok(),
            "Bulk insert should succeed - collection gets created automatically"
        );
    }

    /// Test bulk insert with invalid document ID.
    ///
    /// This test verifies that bulk insert fails when one of the document IDs is invalid,
    /// covering the error handling in bulk_insert.
    #[tokio::test]
    async fn test_bulk_insert_invalid_document_id() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let json_file = temp_dir.path().join("invalid_data.json");

        // Create JSON file with invalid document ID (empty string)
        let test_data = json!({"": {"name": "Alice"}, "user2": {"name": "Bob"}});
        std::fs::write(
            &json_file,
            serde_json::to_string_pretty(&test_data).unwrap(),
        )
        .unwrap();

        // Setup store and collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        let _collection = store
            .collection_with_config("test_collection", None)
            .await
            .unwrap();

        // Test bulk insert with invalid ID
        let args = InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         None,
            data:       None,
            bulk:       Some(json_file.to_string_lossy().to_string()),
            passphrase: None,
            wal:        WalArgs {
                wal_max_file_size:   None,
                wal_format:          None,
                wal_compression:     None,
                wal_max_records:     None,
                wal_write_mode:      None,
                wal_verify_mode:     None,
                wal_auto_verify:     None,
                wal_enable_recovery: None,
            },
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
        assert!(
            result.is_err(),
            "Bulk insert should fail with invalid document ID"
        );
    }
}
