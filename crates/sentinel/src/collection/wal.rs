use sentinel_wal::WalDocumentOps;

use super::collection::Collection;

#[async_trait::async_trait]
impl WalDocumentOps for Collection {
    async fn get_document(&self, id: &str) -> sentinel_wal::Result<Option<serde_json::Value>> {
        self.get(id)
            .await
            .map(|opt| opt.map(|d| d.data().clone()))
            .map_err(|e| {
                sentinel_wal::WalError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("{}", e),
                ))
            })
    }

    async fn apply_operation(
        &self,
        entry_type: &sentinel_wal::EntryType,
        id: &str,
        data: Option<serde_json::Value>,
    ) -> sentinel_wal::Result<()> {
        match *entry_type {
            sentinel_wal::EntryType::Insert => {
                if let Some(data) = data {
                    self.insert(id, data).await.map_err(|e| {
                        sentinel_wal::WalError::Io(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("{}", e),
                        ))
                    })
                }
                else {
                    Err(sentinel_wal::WalError::InvalidEntry(
                        "Insert operation missing data".to_string(),
                    ))
                }
            },
            sentinel_wal::EntryType::Update => {
                if let Some(data) = data {
                    self.update(id, data).await.map_err(|e| {
                        sentinel_wal::WalError::Io(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("{}", e),
                        ))
                    })
                }
                else {
                    Err(sentinel_wal::WalError::InvalidEntry(
                        "Update operation missing data".to_string(),
                    ))
                }
            },
            sentinel_wal::EntryType::Delete => {
                self.delete(id).await.map_err(|e| {
                    sentinel_wal::WalError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("{}", e),
                    ))
                })
            },
            _ => Ok(()), // Other operations not handled here
        }
    }
}