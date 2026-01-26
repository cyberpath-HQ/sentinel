use clap::Args;

/// Arguments for the store list-collections command.
#[derive(Args)]
pub struct ListCollectionsArgs {
    /// Path to the Sentinel store
    #[arg(short, long)]
    pub path:       String,
    /// Passphrase for decrypting the signing key
    #[arg(long)]
    pub passphrase: Option<String>,
}

/// Run the store list-collections command.
pub async fn run(args: ListCollectionsArgs) -> sentinel_dbms::Result<()> {
    let store = sentinel_dbms::Store::new_with_config(
        &args.path,
        args.passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let collections = store.list_collections().await?;
    for collection in collections {
        println!("{}", collection);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_list_collections_empty_store() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create empty store
        let _store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        // Give time for event processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let args = ListCollectionsArgs {
            path:       store_path,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_collections_with_collections() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create store and collections
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let _collection1 = store
            .collection_with_config("collection1", None)
            .await
            .unwrap();
        let _collection2 = store
            .collection_with_config("collection2", None)
            .await
            .unwrap();

        // Give time for event processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let args = ListCollectionsArgs {
            path:       store_path,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }
}
