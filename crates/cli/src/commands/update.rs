use clap::Args;
use serde_json::Value;
use tracing::{error, info};
use sentinel_dbms::{CollectionWalConfig, WalFailureMode};
use crate::commands::WalArgs;

/// Arguments for the update command.
#[derive(Args, Clone)]
pub struct UpdateArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path: String,
    /// Collection name
    #[arg(short, long)]
    pub collection: String,
    /// Document ID
    #[arg(short, long)]
    pub id:         String,
    /// JSON data (as string)
    #[arg(short, long)]
    pub data:       String,
    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal:        WalArgs,
}

/// Update an existing document in a Sentinel collection.
///
/// This function replaces the entire document with the specified ID with new JSON data.
/// It validates the JSON format and handles any errors during the update process.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for update.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::update::{run, UpdateArgs};
///
/// let args = UpdateArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     id:         "user1".to_string(),
///     data:       r#"{"name": "Bob"}"#.to_string(),
///     wal:        WalArgs::default(),
/// };
/// run(args, &WalArgs::default()).await?;
/// ```
pub async fn run(args: UpdateArgs, global_wal: &WalArgs) -> sentinel_dbms::Result<()> {
    info!(
        "Updating document '{}' in collection '{}' in store {}",
        args.id, args.collection, args.store_path
    );
    let store =
        sentinel_dbms::Store::new_with_config(&args.store_path, None, sentinel_dbms::StoreWalConfig::default()).await?;
    let wal_config = build_collection_wal_config(&args, global_wal);
    let coll = store
        .collection_with_config(&args.collection, wal_config)
        .await?;
    let value: Value = match serde_json::from_str(&args.data) {
        Ok(v) => v,
        Err(e) => {
            error!("Invalid JSON data: {}", e);
            return Err(sentinel_dbms::SentinelError::Json {
                source: e,
            });
        },
    };
    match coll.update(&args.id, value).await {
        Ok(_) => {
            info!("Document '{}' updated successfully", args.id);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to update document '{}' in collection '{}' in store {}: {}",
                args.id, args.collection, args.store_path, e
            );
            Err(e)
        },
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    /// Test successful document update.
    ///
    /// This test inserts a document, then updates it with new data.
    #[tokio::test]
    async fn test_update_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init, create collection, insert
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
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

        let insert_args = crate::commands::insert::InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id: Some("doc1".to_string()),
            data: Some(r#"{"name": "Alice", "age": 30}"#.to_string()),
            ..Default::default()
        };
        crate::commands::insert::run(insert_args, &WalArgs::default()).await.unwrap();

        // Now update
        let args = UpdateArgs {
            store_path:          store_path.to_string_lossy().to_string(),
            collection:          "test_collection".to_string(),
            id:                  "doc1".to_string(),
            data:                r#"{"name": "Alice", "age": 31}"#.to_string(),
            wal:             WalArgs::default(),
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
        assert!(
            result.is_ok(),
            "Update should succeed for existing document"
        );
    }

    /// Test update with invalid JSON.
    ///
    /// This test checks that update fails with malformed JSON.
    #[tokio::test]
    async fn test_update_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init, create collection, insert
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
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

        let insert_args = crate::commands::insert::InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id: Some("doc1".to_string()),
            data: Some(r#"{"name": "Alice"}"#.to_string()),
            ..Default::default()
        };
        crate::commands::insert::run(insert_args, &WalArgs::default()).await.unwrap();

        let args = UpdateArgs {
            store_path:          store_path.to_string_lossy().to_string(),
            collection:          "test_collection".to_string(),
            id:                  "doc1".to_string(),
            data:                r#"{"name": "Alice", "age": }"#.to_string(), // Invalid
            wal:             WalArgs::default(),
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
        assert!(result.is_err(), "Update should fail with invalid JSON");
    }

    /// Test update non-existent document.
    ///
    /// This test verifies that update can handle non-existent documents
    /// (may create or fail depending on implementation).
    #[tokio::test]
    async fn test_update_non_existent_document() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init and create collection
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
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

        let args = UpdateArgs {
            store_path:          store_path.to_string_lossy().to_string(),
            collection:          "test_collection".to_string(),
            id:                  "non_existent".to_string(),
            data:                r#"{"name": "Bob"}"#.to_string(),
            wal:             WalArgs::default(),
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
        // Update should fail for non-existent document
        assert!(
            result.is_err(),
            "Update should fail for non-existent document"
        );
    }

    /// Test update in non-existent collection.
    ///
    /// This test checks that update creates the collection if it does not exist.
    #[tokio::test]
    async fn test_update_non_existent_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store only
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
            ..Default::default()
        };
        crate::commands::init::run(init_args).await.unwrap();

        let args = UpdateArgs {
            store_path:          store_path.to_string_lossy().to_string(),
            collection:          "non_existent".to_string(),
            id:                  "doc1".to_string(),
            data:                r#"{"name": "Bob"}"#.to_string(),
            wal:             WalArgs::default(),
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
        assert!(
            result.is_err(),
            "Update should fail for non-existent document even if collection is created"
        );
    }

    /// Test update with read-only collection.
    ///
    /// This test verifies that update fails when the collection directory
    /// is read-only, covering the error branch.
    #[tokio::test]
    async fn test_update_readonly_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init store and create collection
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
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

        let args = UpdateArgs {
            store_path:          store_path.to_string_lossy().to_string(),
            collection:          "test_collection".to_string(),
            id:                  "doc1".to_string(),
            data:                r#"{"name": "Bob"}"#.to_string(),
            wal:             WalArgs::default(),
        };

        let global_wal = WalArgs::default();
        let result = run(args, &global_wal).await;
        assert!(
            result.is_err(),
            "Update should fail on read-only collection"
        );

        // Restore permissions for cleanup
        let mut perms = std::fs::metadata(&collection_path).unwrap().permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(&collection_path, perms).unwrap();
    }
}

/// Build collection WAL config from CLI args
fn build_collection_wal_config(args: &UpdateArgs, global_wal: &WalArgs) -> Option<CollectionWalConfig> {
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
            write_mode:            args.wal.wal_write_mode.or(global_wal.wal_write_mode).unwrap_or(WalFailureMode::Strict),
            verification_mode:     args.wal.wal_verify_mode.or(global_wal.wal_verify_mode).unwrap_or(WalFailureMode::Warn),
            auto_verify:           args.wal.wal_auto_verify.or(global_wal.wal_auto_verify).unwrap_or(false),
            enable_recovery:       args.wal.wal_enable_recovery.or(global_wal.wal_enable_recovery).unwrap_or(true),
            max_wal_size_bytes:    args.wal.wal_max_file_size.or(global_wal.wal_max_file_size),
            compression_algorithm: args.wal.wal_compression.or(global_wal.wal_compression),
            max_records_per_file:  args.wal.wal_max_records.or(global_wal.wal_max_records),
            format:                args.wal.wal_format.or(global_wal.wal_format).unwrap_or_default(),
        })
    }
    else {
        None
    }
}
