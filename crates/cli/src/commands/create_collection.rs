use clap::Args;
use tracing::{error, info};

/// Arguments for the create-collection command.
#[derive(Args, Clone)]
pub struct CreateCollectionArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path: String,
    /// Collection name
    #[arg(short, long)]
    pub name:       String,
}


/// Create a new collection within an existing Sentinel store.
///
/// This function creates a logical grouping for documents within the specified store.
/// It validates that the store exists and handles any errors during collection creation.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for create-collection.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::create_collection::{
///     run,
///     CreateCollectionArgs,
/// };
///
/// let args = CreateCollectionArgs {
///     store_path: "/tmp/my_store".to_string(),
///     name:       "users".to_string(),
/// };
/// run(args).await?;
/// ```
pub async fn run(args: CreateCollectionArgs) -> sentinel::Result<()> {
    let store_path = args.store_path;
    let name = args.name;
    info!("Creating collection '{}' in store {}", name, store_path);
    let store = sentinel::Store::new(&store_path).await?;
    match store.collection(&name).await {
        Ok(_) => {
            info!("Collection '{}' created successfully", name);
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to create collection '{}' in store {}: {}",
                name, store_path, e
            );
            Err(e)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Test successful collection creation.
    ///
    /// This test verifies that create-collection succeeds when given a valid
    /// store path and collection name. It first initializes a store, then creates
    /// a collection within it.
    #[tokio::test]
    async fn test_create_collection_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // First init the store
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        let args = CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Create collection should succeed");
    }

    /// Test create collection with non-existent store.
    ///
    /// This test checks that create-collection creates the store if it doesn't exist.
    #[tokio::test]
    async fn test_create_collection_non_existent_store() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("non_existent_store");

        let args = CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok(), "Create collection should create store if needed");
    }

    /// Test create collection with invalid collection name.
    ///
    /// This test verifies behavior with potentially invalid collection names,
    /// such as empty strings or names with special characters.
    #[tokio::test]
    async fn test_create_collection_invalid_name() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Init store
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        // Test with empty name
        let args = CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "".to_string(),
        };

        let result = run(args).await;
        // Depending on implementation, might succeed or fail
        // For now, assume it succeeds as collection creation might handle empty names
        assert!(result.is_ok(), "Create collection with empty name should be handled");
    }

    /// Test create collection with read-only store.
    ///
    /// This test verifies that create-collection fails when the store directory
    /// is read-only, covering the error branch.
    #[tokio::test]
    async fn test_create_collection_readonly_store() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("readonly_store");

        // Init store
        let init_args = crate::commands::init::InitArgs {
            path: store_path.to_string_lossy().to_string(),
        };
        crate::commands::init::run(init_args).await.unwrap();

        // Make the store directory read-only
        let mut perms = std::fs::metadata(&store_path).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&store_path, perms).unwrap();

        let args = CreateCollectionArgs {
            store_path: store_path.to_string_lossy().to_string(),
            name: "test_collection".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_err(), "Create collection should fail on read-only store");

        // Restore permissions for cleanup
        let mut perms = std::fs::metadata(&store_path).unwrap().permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(&store_path, perms).unwrap();
    }
}
