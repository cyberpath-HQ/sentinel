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
    /// Document ID
    #[arg(short, long)]
    pub id:         String,
    /// JSON data (as string)
    #[arg(short, long)]
    pub data:       String,
    /// Passphrase for decrypting the signing key
    #[arg(long)]
    pub passphrase: Option<String>,
}

/// Insert a new document into a Sentinel collection.
///
/// This function parses the provided JSON data and inserts it as a new document
/// with the specified ID into the given collection. It validates the JSON format
/// and handles any errors during the insertion process.
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
/// let args = InsertArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     id:         "user1".to_string(),
///     data:       r#"{"name": "Alice"}"#.to_string(),
///     passphrase: None,
/// };
/// run(args).await?;
/// ```
pub async fn run(args: InsertArgs) -> sentinel_dbms::Result<()> {
    let store_path = args.store_path;
    let collection = args.collection;
    let id = args.id;
    let data = args.data;
    info!(
        "Inserting document '{}' into collection '{}' in store {}",
        id, collection, store_path
    );
    let store = sentinel_dbms::Store::new(&store_path, args.passphrase.as_deref()).await?;
    let coll = store.collection(&collection).await?;
    let value: Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(e) => {
            error!("Invalid JSON data: {}", e);
            return Err(sentinel_dbms::SentinelError::Json {
                source: e,
            });
        },
    };
    match coll.insert(&id, value).await {
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

#[cfg(test)]
mod tests {
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
            id:         "doc1".to_string(),
            data:       r#"{"name": "Alice", "age": 30}"#.to_string(),
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
            id:         "doc1".to_string(),
            data:       r#"{"name": "Alice", "age": }"#.to_string(), // Invalid JSON
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_err(), "Insert should fail with invalid JSON");
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
            id:         "doc1".to_string(),
            data:       r#"{"name": "Alice"}"#.to_string(),
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
            id:         "doc1".to_string(),
            data:       "{}".to_string(),
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
            id:         "doc1".to_string(),
            data:       r#"{"users": [{"name": "Alice"}, {"name": "Bob"}], "metadata": {"version": 1}}"#.to_string(),
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
            id:         "doc1".to_string(),
            data:       r#"{"name": "Alice"}"#.to_string(),
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
}
