use clap::Args;
use tracing::{error, info};

use crate::commands::WalArgs;

/// Arguments for the collection delete command.
#[derive(Args, Clone)]
pub struct DeleteArgs {
    /// Document ID
    #[arg(short, long)]
    pub id:  String,
    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal: WalArgs,
}

/// Delete a document from a Sentinel collection.
///
/// This function removes the document with the specified ID from the given collection.
/// It handles any errors that may occur during the deletion process.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection` - Collection name
/// * `passphrase` - Optional passphrase for decrypting the signing key
/// * `args` - The parsed command-line arguments for collection delete.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::collection::delete::{run, DeleteArgs};
///
/// let args = DeleteArgs {
///     id:  String::from("user1"),
///     wal: WalArgs::default(),
/// };
/// run(
///     String::from("/tmp/my_store"),
///     String::from("users"),
///     None,
///     args,
/// )
/// .await?;
/// ```

pub async fn run(
    store_path: String,
    collection: String,
    passphrase: Option<String>,
    args: DeleteArgs,
) -> sentinel_dbms::Result<()> {
    let id = args.id;
    info!(
        "Deleting document '{}' from collection '{}' in store {}",
        id, collection, store_path
    );
    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let coll = store
        .collection_with_config(&collection, Some(args.wal.to_overrides()))
        .await?;
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

    #[tokio::test]
    async fn test_delete_existing_document() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";
        let doc_id = "doc1";

        // Initialize store and create collection with document
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let docs = vec![(doc_id, serde_json::json!({"name": "Alice"}))];
        collection.bulk_insert(docs).await.unwrap();

        let args = DeleteArgs {
            id:  doc_id.to_string(),
            wal: WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        assert!(result.is_ok());

        // Verify document is deleted
        let doc = collection.get(doc_id).await.unwrap();
        assert!(doc.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_document() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");
        let collection_name = "test_collection";
        let doc_id = "nonexistent";

        // Initialize store and create collection
        let store = sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default())
            .await
            .unwrap();

        let _collection = store
            .collection_with_config(collection_name, None)
            .await
            .unwrap();

        let args = DeleteArgs {
            id:  doc_id.to_string(),
            wal: WalArgs::default(),
        };

        let result = run(
            store_path.to_string_lossy().to_string(),
            collection_name.to_string(),
            None,
            args,
        )
        .await;

        // Should succeed even if document doesn't exist (idempotent)
        assert!(result.is_ok());
    }
}
