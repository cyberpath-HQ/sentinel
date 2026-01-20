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

    println!("Store Statistics:");
    println!("  Root Path: {}", store.root_path().display());
    println!("  Created At: {}", store.created_at());
    println!("  Last Accessed At: {}", store.last_accessed_at());
    println!("  Total Documents: {}", store.total_documents());
    println!("  Total Size (bytes): {}", store.total_size_bytes());
    println!("  Collection Count: {}", store.collection_count());

    Ok(())
}