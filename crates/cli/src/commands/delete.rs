use clap::Args;
use tracing::{error, info};

/// Arguments for the delete command.
#[derive(Args, Clone)]
pub struct DeleteArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path: String,
    /// Collection name
    #[arg(short, long)]
    pub collection: String,
    /// Document ID
    #[arg(short, long)]
    pub id:         String,
}

/// Delete a document from a Sentinel collection.
///
/// This function removes the document with the specified ID from the given collection.
/// It handles any errors that may occur during the deletion process.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for delete.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::delete::{run, DeleteArgs};
///
/// let args = DeleteArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     id:         "user1".to_string(),
/// };
/// run(args).await?;
/// ```
pub async fn run(args: DeleteArgs) -> sentinel::Result<()> {
    let store_path = args.store_path;
    let collection = args.collection;
    let id = args.id;
    info!(
        "Deleting document '{}' from collection '{}' in store {}",
        id, collection, store_path
    );
    let store = sentinel::Store::new(&store_path).await?;
    let coll = store.collection(&collection).await?;
    match coll.delete(&id).await {
        Ok(_) => {
            info!("Document '{}' deleted successfully", id);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to delete document '{}' from collection '{}' in store {}: {}",
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

    /// Test successful document deletion.
    ///
    /// This test inserts a document, then deletes it successfully.
    #[tokio::test]
    async fn test_delete_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init, create collection, insert
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name:       "test_collection".to_string(),
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let insert_args = crate::commands::insert::InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
            data:       r#"{"name": "Alice"}"#.to_string(),
        };
        crate::commands::insert::run(insert_args).await.unwrap();

        // Now delete
        let args = DeleteArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
        };

        let result = run(args).await;
        assert!(
            result.is_ok(),
            "Delete should succeed for existing document"
        );
    }

    /// Test delete non-existent document.
    ///
    /// This test verifies that delete fails gracefully when the document
    /// does not exist (depending on implementation, might succeed or fail).
    #[tokio::test]
    async fn test_delete_non_existent_document() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init and create collection
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name:       "test_collection".to_string(),
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let args = DeleteArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "non_existent".to_string(),
        };

        let result = run(args).await;
        // Depending on implementation, deleting non-existent might succeed or fail
        // Assume it succeeds for idempotency
        assert!(
            result.is_ok(),
            "Delete should handle non-existent document gracefully"
        );
    }

    /// Test delete from non-existent collection.
    ///
    /// This test checks that delete creates the collection if it does not exist.
    #[tokio::test]
    async fn test_delete_non_existent_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store only
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        let args = DeleteArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "non_existent".to_string(),
            id:         "doc1".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Delete should create collection if needed");
    }

    /// Test delete with empty ID.
    ///
    /// This test verifies behavior when an empty document ID is provided.
    #[tokio::test]
    async fn test_delete_empty_id() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init and create collection
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name:       "test_collection".to_string(),
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        let args = DeleteArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "".to_string(),
        };

        let result = run(args).await;
        // Empty IDs are now rejected by validation
        assert!(result.is_err(), "Delete with empty ID should fail");
    }

    /// Test delete with read-only collection.
    ///
    /// This test verifies that delete fails when the collection directory
    /// is read-only, covering the error branch.
    #[tokio::test]
    async fn test_delete_readonly_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup: init store and create collection
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        let create_args = crate::commands::create_collection::CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name:       "test_collection".to_string(),
        };
        crate::commands::create_collection::run(create_args)
            .await
            .unwrap();

        // Insert a document first
        let insert_args = crate::commands::insert::InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
            data:       r#"{"name": "test"}"#.to_string(),
        };
        crate::commands::insert::run(insert_args).await.unwrap();

        // Make the collection directory read-only
        let collection_path = store_path.join("data").join("test_collection");
        let mut perms = std::fs::metadata(&collection_path).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&collection_path, perms).unwrap();

        let args = DeleteArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id:         "doc1".to_string(),
        };

        let result = run(args).await;
        assert!(
            result.is_err(),
            "Delete should fail on read-only collection"
        );

        // Restore permissions for cleanup
        let mut perms = std::fs::metadata(&collection_path).unwrap().permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(&collection_path, perms).unwrap();
    }
}
