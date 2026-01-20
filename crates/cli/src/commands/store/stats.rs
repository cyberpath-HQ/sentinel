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
