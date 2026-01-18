use clap::Args;
use sentinel_dbms::{
    futures::{pin_mut, StreamExt as _},
    VerificationMode,
    VerificationOptions,
};
use tracing::{error, info};

/// Arguments for list command.
#[derive(Args, Clone, Default)]
pub struct ListArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path:       String,
    /// Collection name
    #[arg(short, long)]
    pub collection:       String,
    /// Passphrase for decrypting signing key
    #[arg(long)]
    pub passphrase:       Option<String>,
    /// Verify document signature (default: true)
    #[arg(long, default_value = "true")]
    pub verify_signature: bool,
    /// Verify document hash (default: true)
    #[arg(long, default_value = "true")]
    pub verify_hash:      bool,
    /// Signature verification mode: strict, warn, or silent (default: strict)
    #[arg(long, default_value = "strict")]
    pub signature_mode:   String,
    /// Hash verification mode: strict, warn, or silent (default: strict)
    #[arg(long, default_value = "strict")]
    pub hash_mode:        String,
}

impl ListArgs {
    /// Convert CLI arguments to verification options.
    fn to_verification_options(&self) -> Result<VerificationOptions, String> {
        let signature_verification_mode = VerificationMode::from_str(&self.signature_mode).ok_or_else(|| {
            format!(
                "Invalid signature verification mode: {}",
                self.signature_mode
            )
        })?;

        let hash_verification_mode = VerificationMode::from_str(&self.hash_mode)
            .ok_or_else(|| format!("Invalid hash verification mode: {}", self.hash_mode))?;

        Ok(VerificationOptions {
            verify_signature: self.verify_signature,
            verify_hash: self.verify_hash,
            signature_verification_mode,
            hash_verification_mode,
        })
    }
}

/// List all documents in a Sentinel collection.
///
/// This function retrieves and prints IDs of all documents in specified collection.
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
///     store_path:       "/tmp/my_store".to_string(),
///     collection:       "users".to_string(),
///     passphrase:       None,
///     verify_signature: true,
///     verify_hash:      true,
///     signature_mode:   "strict".to_string(),
///     hash_mode:        "strict".to_string(),
/// };
/// run(args).await?;
/// ```
pub async fn run(args: ListArgs) -> sentinel_dbms::Result<()> {
    let store_path = args.store_path;
    let collection_name = args.collection;
    info!(
        "Listing documents in collection '{}' in store {}",
        collection_name, store_path
    );
    let store = sentinel_dbms::Store::new(&store_path, args.passphrase.as_deref()).await?;
    let coll = store.collection(&collection_name).await?;

    let verification_options = args.to_verification_options().map_err(|e| {
        sentinel_dbms::SentinelError::ConfigError {
            message: e,
        }
    })?;

    let stream = coll.all_with_verification(&verification_options);
    pin_mut!(stream);

    let mut count: usize = 0;
    // Process stream item by item to avoid loading all IDs into memory
    while let Some(item) = stream.next().await {
        match item {
            Ok(id) => {
                #[allow(clippy::print_stdout, reason = "CLI output")]
                {
                    println!("{}", &id);
                }
                count = count.saturating_add(1);
            },
            Err(e) => {
                error!(
                    "Failed to list documents in collection '{}' in store {}: {}",
                    collection_name, store_path, e
                );
                return Err(e);
            },
        }
    }

    info!(
        "Found {} documents in collection '{}'",
        count, collection_name
    );
    Ok(())
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
            store_path:       store_path.to_string_lossy().to_string(),
            collection:       "test_collection".to_string(),
            passphrase:       None,
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            hash_mode:        "strict".to_string(),
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
            store_path:       store_path.to_string_lossy().to_string(),
            collection:       "test_collection".to_string(),
            passphrase:       None,
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            hash_mode:        "strict".to_string(),
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
            store_path:       store_path.to_string_lossy().to_string(),
            collection:       "nonexistent".to_string(),
            passphrase:       None,
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            hash_mode:        "strict".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok()); // Should succeed and create collection
    }

    /// Test list with unreadable collection directory.
    ///
    /// This test verifies that list fails when collection directory
    /// is unreadable, covering the error handling in stream processing.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_list_unreadable_collection() {
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

        // Insert a document first
        let insert_args = crate::commands::insert::InsertArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            id: Some("doc1".to_string()),
            data: Some(r#"{"name": "test"}"#.to_string()),
            ..Default::default()
        };
        crate::commands::insert::run(insert_args).await.unwrap();

        // Make collection directory unreadable (no read permission)
        let collection_path = store_path.join("data").join("test_collection");
        let mut perms = std::fs::metadata(&collection_path).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o000); // No permissions
        }
        std::fs::set_permissions(&collection_path, perms).unwrap();

        let args = ListArgs {
            store_path:       store_path.to_string_lossy().to_string(),
            collection:       "test_collection".to_string(),
            passphrase:       None,
            verify_signature: true,
            verify_hash:      true,
            signature_mode:   "strict".to_string(),
            hash_mode:        "strict".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_err(), "List should fail on unreadable collection");

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
