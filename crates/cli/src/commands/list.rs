use clap::Args;
use tracing::{error, info};

/// Arguments for the list command.
#[derive(Args, Clone, Default)]
pub struct ListArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path: String,
    /// Collection name
    #[arg(short, long)]
    pub collection: String,
    /// Passphrase for decrypting the signing key
    #[arg(long)]
    pub passphrase: Option<String>,
}

/// List all documents in a Sentinel collection.
///
/// This function retrieves and prints the IDs of all documents in the specified collection.
/// The IDs are printed one per line to stdout.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for list.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::list::{run, ListArgs};
///
/// let args = ListArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     passphrase: None,
/// };
/// run(args).await?;
/// ```
pub async fn run(args: ListArgs) -> sentinel_dbms::Result<()> {
    let store_path = args.store_path;
    let collection = args.collection;
    info!(
        "Listing documents in collection '{}' in store {}",
        collection, store_path
    );
    let store = sentinel_dbms::Store::new(&store_path, args.passphrase.as_deref()).await?;
    let coll = store.collection(&collection).await?;
    match coll.list().await {
        Ok(ids) => {
            info!(
                "Found {} documents in collection '{}'",
                ids.len(),
                collection
            );
            for id in ids {
                #[allow(clippy::print_stdout, reason = "CLI output")]
                {
                    println!("{}", id);
                }
            }
            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to list documents in collection '{}' in store {}: {}",
                collection, store_path, e
            );
            Err(e)
        },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    /// Test successful document listing.
    ///
    /// This test inserts multiple documents and then lists them successfully.
    #[tokio::test]
    async fn test_list_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Insert test documents
        collection
            .insert("doc1", json!({"name": "Alice"}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"name": "Bob"}))
            .await
            .unwrap();
        collection
            .insert("doc3", json!({"name": "Charlie"}))
            .await
            .unwrap();

        // Test list command
        let args = ListArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test listing empty collection.
    ///
    /// This test verifies that listing an empty collection works correctly.
    #[tokio::test]
    async fn test_list_empty_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let _collection = store.collection("test_collection").await.unwrap();

        // Test list command on empty collection
        let args = ListArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test listing non-existent collection.
    ///
    /// This test verifies that attempting to list a non-existent collection
    /// creates the collection and returns an empty list.
    #[tokio::test]
    async fn test_list_nonexistent_collection() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store
        let _store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();

        // Test list command on non-existent collection
        let args = ListArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "nonexistent".to_string(),
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok()); // Should succeed and create the collection
    }
}
