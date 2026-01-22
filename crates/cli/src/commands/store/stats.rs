use clap::Args;

/// Arguments for the store stats command.
#[derive(Args)]
pub struct StatsArgs {
    /// Path to the Sentinel store
    #[arg(short, long)]
    pub path:       String,
    /// Passphrase for decrypting the signing key
    #[arg(long)]
    pub passphrase: Option<String>,
}

/// Run the store stats command.
pub async fn run(args: StatsArgs) -> sentinel_dbms::Result<()> {
    let store = sentinel_dbms::Store::new_with_config(
        &args.path,
        args.passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;

    tracing::info!("Store Statistics:");
    tracing::info!("  Root Path: {}", store.root_path().display());
    tracing::info!("  Created At: {}", store.created_at());
    tracing::info!("  Last Accessed At: {}", store.last_accessed_at());
    tracing::info!("  Total Documents: {}", store.total_documents());
    tracing::info!("  Total Size (bytes): {}", store.total_size_bytes());
    tracing::info!("  Collection Count: {}", store.collection_count());

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_stats_empty_store() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create empty store
        let _store = sentinel_dbms::Store::new_with_config(
            &store_path,
            None,
            sentinel_dbms::StoreWalConfig::default(),
        )
        .await
        .unwrap();

        // Give time for event processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let args = StatsArgs {
            path: store_path,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stats_store_with_data() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_string_lossy().to_string();

        // Create store and add some data
        let store = sentinel_dbms::Store::new_with_config(
            &store_path,
            None,
            sentinel_dbms::StoreWalConfig::default(),
        )
        .await
        .unwrap();

        let collection = store
            .collection_with_config("test_collection", None)
            .await
            .unwrap();

        // Add some documents
        collection
            .insert("doc1", json!({"name": "Alice", "age": 30}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"name": "Bob", "age": 25}))
            .await
            .unwrap();

        // Give time for event processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let args = StatsArgs {
            path: store_path,
            passphrase: None,
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }
}
