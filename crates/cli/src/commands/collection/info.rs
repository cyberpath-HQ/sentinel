use clap::Args;

/// Arguments for collection info command.
#[derive(Args)]
pub struct InfoArgs {
    /// Output format: table (default) or json
    #[arg(long, default_value = "table")]
    pub format: String,
}

/// Execute collection info command.
///
/// Displays metadata and statistics for the specified collection.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection_name` - Name of the collection
/// * `passphrase` - Optional passphrase for decrypting signing key
/// * `args` - Info command arguments
///
/// # Returns
/// Returns `Ok(())` on success.
pub async fn run(
    store_path: String,
    collection_name: String,
    passphrase: Option<String>,
    args: InfoArgs,
) -> sentinel_dbms::Result<()> {
    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let collection = store.collection_with_config(&collection_name, None).await?;

    match args.format.as_str() {
        "table" => {
            println!("Collection Information");
            println!("====================");
            println!("Name:              {}", collection.name());
            println!(
                "Created At:        {}",
                collection.created_at().format("%Y-%m-%d %H:%M:%S UTC")
            );
            println!(
                "Updated At:        {}",
                collection.updated_at().format("%Y-%m-%d %H:%M:%S UTC")
            );

            if let Some(checkpoint) = collection.last_checkpoint_at() {
                println!(
                    "Last Checkpoint:   {}",
                    checkpoint.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
            else {
                println!("Last Checkpoint:   Never");
            }

            println!("Total Documents:   {}", collection.total_documents());
            println!(
                "Total Size:        {} bytes ({:.2} MB)",
                collection.total_size_bytes(),
                collection.total_size_bytes() as f64 / 1_000_000.0
            );
        },
        _ => {
            return Err(sentinel_dbms::SentinelError::Internal {
                message: format!("Invalid format: {}. Use 'table'", args.format),
            });
        },
    }

    Ok(())
}
