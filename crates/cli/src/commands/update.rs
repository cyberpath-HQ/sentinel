use clap::Args;
use serde_json::Value;
use tracing::{error, info};

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
/// };
/// run(args).await?;
/// ```
pub async fn run(args: UpdateArgs) -> sentinel::Result<()> {
    let store_path = args.store_path;
    let collection = args.collection;
    let id = args.id;
    let data = args.data;
    info!(
        "Updating document '{}' in collection '{}' in store {}",
        id, collection, store_path
    );
    let store = sentinel::Store::new(&store_path, None).await?;
    let coll = store.collection(&collection).await?;
    let value: Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(e) => {
            error!("Invalid JSON data: {}", e);
            return Err(sentinel::SentinelError::Json {
                source: e,
            });
        },
    };
    match coll.update(&id, value).await {
        Ok(_) => {
            info!("Document '{}' updated successfully", id);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to update document '{}' in collection '{}' in store {}: {}",
                id, collection, store_path, e
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
            id: "doc1".to_string(),
            data: r#"{"name": "Alice", "age": 30}"#.to_string(),
            ..Default::default()
        };
        crate::commands::insert::run(insert_args).await.unwrap();

        // Now update
        let args = UpdateArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
            data:       r#"{"name": "Alice", "age": 31}"#.to_string(),
        };

        let result = run(args).await;
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
            id: "doc1".to_string(),
            data: r#"{"name": "Alice"}"#.to_string(),
            ..Default::default()
        };
        crate::commands::insert::run(insert_args).await.unwrap();

        let args = UpdateArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
            data:       r#"{"name": "Alice", "age": }"#.to_string(), // Invalid
        };

        let result = run(args).await;
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
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "non_existent".to_string(),
            data:       r#"{"name": "Bob"}"#.to_string(),
        };

        let result = run(args).await;
        // Depending on implementation, may succeed or fail
        assert!(result.is_ok(), "Update should handle non-existent document");
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
            store_path: store_path.to_string_lossy().to_string(),
            collection: "non_existent".to_string(),
            id:         "doc1".to_string(),
            data:       r#"{"name": "Bob"}"#.to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Update should create collection if needed");
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
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
            data:       r#"{"name": "Bob"}"#.to_string(),
        };

        let result = run(args).await;
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
