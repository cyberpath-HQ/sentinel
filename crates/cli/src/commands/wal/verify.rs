//! WAL verification command.

use clap::Args;
use tracing::{error, info, warn};

/// Arguments for the WAL verify command.
#[derive(Args)]
pub struct VerifyArgs;

/// Execute the WAL verification operation.
pub async fn run(store_path: String, collection: Option<String>, _args: VerifyArgs) -> sentinel_dbms::Result<()> {
    use sentinel_dbms::wal::ops::{CollectionWalOps as _, StoreWalOps as _};

    let store =
        sentinel_dbms::Store::new_with_config(&store_path, None, sentinel_dbms::StoreWalConfig::default()).await?;

    if let Some(collection_name) = collection {
        let collection = store.collection_with_config(&collection_name, None).await?;
        info!(
            "Verifying WAL integrity for collection '{}'...",
            collection_name
        );

        let result = collection.verify_against_wal().await?;
        info!(
            "Verification completed for collection '{}'",
            collection_name
        );
        info!("  Entries processed: {}", result.entries_processed);
        info!("  Passed: {}", result.passed);

        if !result.issues.is_empty() {
            warn!("  Issues found: {}", result.issues.len());
            for issue in &result.issues {
                warn!("    - {}", issue.description);
            }
            if result.issues.iter().any(|i| i.is_critical) {
                error!(
                    "Critical verification issues found in collection '{}'",
                    collection_name
                );
                return Err(sentinel_dbms::SentinelError::ConfigError {
                    message: format!(
                        "Critical verification issues found in collection '{}'",
                        collection_name
                    ),
                });
            }
        }
    }
    else {
        info!("Verifying WAL integrity for all collections...");
        let issues = store.verify_all_collections().await?;

        if issues.is_empty() {
            info!("All collections verified successfully");
        }
        else {
            error!("Verification issues found");
            let mut has_critical = false;
            for (collection_name, collection_issues) in issues {
                warn!(
                    "  Collection '{}': {} issues",
                    collection_name,
                    collection_issues.len()
                );
                for issue in collection_issues {
                    warn!("    - {}", issue.description);
                    if issue.is_critical {
                        has_critical = true;
                    }
                }
            }
            if has_critical {
                error!("Critical verification issues found");
                return Err(sentinel_dbms::SentinelError::ConfigError {
                    message: "Critical verification issues found".to_owned(),
                });
            }
        }
    }

    Ok(())
}
