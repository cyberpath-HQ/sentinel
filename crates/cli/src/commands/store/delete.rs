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
