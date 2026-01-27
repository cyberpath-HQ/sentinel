use clap::Args;

/// Arguments for the store delete-collection command.
#[derive(Args)]
pub struct DeleteCollectionArgs {
    /// Path to the Sentinel store
    #[arg(short, long)]
    pub path:       String,
    /// Collection name
    #[arg(short, long)]
    pub collection: String,
    /// Passphrase for decrypting the signing key
    #[arg(long)]
    pub passphrase: Option<String>,
}

/// Run the store delete-collection command.
pub async fn run(args: DeleteCollectionArgs) -> sentinel_dbms::Result<()> {
    let store = sentinel_dbms::Store::new_with_config(
        &args.path,
        args.passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    store.delete_collection(&args.collection).await?;
    tracing::info!("Collection '{}' deleted successfully", args.collection);
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_delete_collection_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();
        let collection_name = "test_collection".to_string();

        // Create store and collection first
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();
        let _collection = store
            .collection_with_config(&collection_name, None)
            .await
            .unwrap();

        // Give time for event processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Now delete the collection
        let args = DeleteCollectionArgs {
            path:       store_path,
            collection: collection_name,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_collection_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();
        let collection_name = "nonexistent_collection".to_string();

        // Create store but not the collection
        let _store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        // Give time for event processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Try to delete non-existent collection
        let args = DeleteCollectionArgs {
            path:       store_path,
            collection: collection_name,
            passphrase: None,
        };

        let result = run(args).await;
        // Note: Deleting a non-existent collection may succeed (idempotent operation)
        // So we just check that it doesn't panic
        assert!(result.is_ok());
    }
}
