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