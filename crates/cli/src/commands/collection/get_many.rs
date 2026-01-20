use clap::Args;
use tracing::info;

/// Arguments for collection get-many command.
#[derive(Args)]
pub struct GetManyArgs {
    /// Document IDs to retrieve (can be used multiple times)
    #[arg(short, long = "id", value_name = "ID")]
    pub ids: Vec<String>,

    /// Output format: json or table
    #[arg(long, default_value = "json")]
    pub format: String,

    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal: crate::commands::WalArgs,
}

/// Execute collection get-many command.
///
/// Retrieves multiple documents from the specified collection by their IDs.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection_name` - Name of the collection
/// * `passphrase` - Optional passphrase for decrypting signing key
/// * `args` - Get-many command arguments
///
/// # Returns
/// Returns `Ok(())` on success.
pub async fn run(
    store_path: String,
    collection_name: String,
    passphrase: Option<String>,
    args: GetManyArgs,
) -> sentinel_dbms::Result<()> {
    if args.ids.is_empty() {
        info!("No document IDs specified");
        return Ok(());
    }

    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let collection = store
        .collection_with_config(&collection_name, Some(args.wal.to_overrides()))
        .await?;

    // Convert Vec<String> to Vec<&str>
    let ids: Vec<&str> = args.ids.iter().map(|s| s.as_str()).collect();

    let documents = collection.get_many(&ids).await?;

    match args.format.as_str() {
        "json" => {
            let results: Vec<serde_json::Value> = documents
                .into_iter()
                .zip(ids.iter())
                .map(|(doc, id)| {
                    if let Some(doc) = doc {
                        serde_json::json!({
                            "id": id,
                            "found": true,
                            "data": doc.data()
                        })
                    }
                    else {
                        serde_json::json!({
                            "id": id,
                            "found": false
                        })
                    }
                })
                .collect();

            println!("{}", serde_json::to_string_pretty(&results)?);
        },
        "table" => {
            println!("{:<30} {:<6} {}", "ID", "Found", "Data Preview");
            println!("{}", "-".repeat(80));

            for (doc, id) in documents.into_iter().zip(ids.iter()) {
                let found = if doc.is_some() { "Yes" } else { "No" };
                let preview = if let Some(doc) = &doc {
                    let data_str = serde_json::to_string(&doc.data())?;
                    if data_str.len() > 40 {
                        format!("{}...", &data_str[.. 37])
                    }
                    else {
                        data_str
                    }
                }
                else {
                    "".to_string()
                };
                println!("{:<30} {:<6} {}", id, found, preview);
            }
        },
        _ => {
            return Err(sentinel_dbms::SentinelError::Internal {
                message: format!("Invalid format: {}. Use 'json' or 'table'", args.format),
            });
        },
    }

    Ok(())
}
