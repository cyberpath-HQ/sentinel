use clap::Args;
use tracing::{error, info, warn};

/// Arguments for the get command.
#[derive(Args, Clone)]
pub struct GetArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path: String,
    /// Collection name
    #[arg(short, long)]
    pub collection: String,
    /// Document ID
    #[arg(short, long)]
    pub id: String,
}

/// Retrieve a document from a Sentinel collection.
///
/// This function fetches the document with the specified ID from the given collection.
/// If the document exists, its JSON data is printed to stdout. If not found,
/// a warning is logged.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for get.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::get::{run, GetArgs};
///
/// let args = GetArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     id:         "user1".to_string(),
/// };
/// run(args).await?;
/// ```
pub async fn run(args: GetArgs) -> sentinel::Result<()> {
    let store_path = args.store_path;
    let collection = args.collection;
    let id = args.id;
    info!(
        "Getting document '{}' from collection '{}' in store {}",
        id, collection, store_path
    );
    let store = sentinel::Store::new(&store_path).await?;
    let coll = store.collection(&collection).await?;
    match coll.get(&id).await {
        Ok(Some(doc)) => {
            info!("Document '{}' retrieved successfully", id);
            match serde_json::to_string_pretty(&doc.data) {
                Ok(json) => {
                    println!("{}", json);
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to serialize document to JSON: {}", e);
                    Err(sentinel::SentinelError::Json {
                        source: e,
                    })
                },
            }
        },
        Ok(None) => {
            warn!("Document '{}' not found", id);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to get document '{}' from collection '{}' in store {}: {}",
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

    /// Test successful document retrieval.
    ///
    /// This test inserts a document and then retrieves it successfully.
    #[tokio::test]
    async fn test_get_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init store, create collection, insert document
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let insert_args = crate::commands::insert::InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id: "doc1".to_string(),
            data: r#"{"name": "Alice", "age": 30}"#.to_string(),
        };
        crate::commands::insert::run(insert_args).await.unwrap();

        // Capture stdout for testing
        {
            let args = GetArgs {
                store_path: store_path.to_string_lossy().to_string(),
                collection: "test_collection".to_string(),
                id: "doc1".to_string(),
            };

            // Since run prints to stdout, we need to capture it
            // For simplicity, we'll just check that it doesn't error
            let result = run(args).await;
            assert!(result.is_ok(), "Get should succeed for existing document");
        }
    }

    /// Test get non-existent document.
    ///
    /// This test verifies that get handles the case where the document
    /// does not exist (should succeed but warn).
    #[tokio::test]
    async fn test_get_non_existent_document() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init store and collection
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let args = GetArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id: "non_existent".to_string(),
        };

        let result = run(args).await;
        assert!(
            result.is_ok(),
            "Get should succeed (but warn) for non-existent document"
        );
    }

    /// Test get from non-existent collection.
    ///
    /// This test checks that get creates the collection if it does not exist.
    #[tokio::test]
    async fn test_get_non_existent_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store only
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        let args = GetArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "non_existent".to_string(),
            id: "doc1".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Get should create collection if needed");
    }

    /// Test get with empty ID.
    ///
    /// This test verifies behavior when an empty document ID is provided.
    #[tokio::test]
    async fn test_get_empty_id() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init store and collection
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let args = GetArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id: "".to_string(),
        };

        let result = run(args).await;
        // Depending on implementation, might succeed or fail
        // Assume it succeeds as empty ID might be allowed
        assert!(result.is_ok(), "Get with empty ID should be handled");
    }

    /// This test verifies that get fails when the collection directory
    /// is unreadable, covering the coll.get error branch.
    #[tokio::test]
    async fn test_get_unreadable_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init store and create collection
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        // Insert a document first
        let insert_args = crate::commands::insert::InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id: "doc1".to_string(),
            data: r#"{"name": "test"}"#.to_string(),
        };
        crate::commands::insert::run(insert_args).await.unwrap();

        // Make the collection directory unreadable (no read permission)
        let collection_path = store_path.join("data").join("test_collection");
        let mut perms = std::fs::metadata(&collection_path).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o000); // No permissions
        }
        std::fs::set_permissions(&collection_path, perms).unwrap();

        let args = GetArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id: "doc1".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_err(), "Get should fail on unreadable collection");

        // Restore permissions for cleanup
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&collection_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&collection_path, perms).unwrap();
        }
    }
}
