//! WAL manager for handling log operations

use std::{path::PathBuf, sync::Arc};

use crc32fast::Hasher as Crc32Hasher;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom},
};
use tracing::{debug, info, warn};

use crate::{LogEntry, Result};

/// Write-Ahead Log manager
#[derive(Debug)]
pub struct WalManager {
    /// Path to the WAL file
    path:     PathBuf,
    /// Current WAL file handle
    file:     Arc<tokio::sync::RwLock<File>>,
    /// Current position in the file
    position: Arc<tokio::sync::RwLock<u64>>,
}

impl WalManager {
    /// Create a new WAL manager
    pub async fn new(path: PathBuf) -> Result<Self> {
        info!("Initializing WAL manager at {:?}", path);

        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)
            .await?;

        let position = file.metadata().await?.len();

        Ok(Self {
            path,
            file: Arc::new(tokio::sync::RwLock::new(file)),
            position: Arc::new(tokio::sync::RwLock::new(position)),
        })
    }

    /// Write a log entry to the WAL
    pub async fn write_entry(&self, entry: LogEntry) -> Result<()> {
        debug!("Writing WAL entry: {:?}", entry.entry_type);

        let bytes = entry.to_bytes()?;
        let mut file = self.file.write().await;
        let mut pos = self.position.write().await;

        file.write_all(&bytes).await?;
        file.flush().await?;

        *pos += bytes.len() as u64;

        debug!("WAL entry written successfully");
        Ok(())
    }

    /// Read all entries from the WAL (for recovery)
    pub async fn read_all_entries(&self) -> Result<Vec<LogEntry>> {
        info!("Reading all WAL entries for recovery");

        let mut file = self.file.write().await;
        file.seek(SeekFrom::Start(0)).await?;

        let mut entries = Vec::new();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let mut offset = 0;
        while offset < buffer.len() {
            // Find the next entry by checking checksums
            let mut entry_end = offset;
            while entry_end + 4 <= buffer.len() {
                let data = &buffer[offset .. entry_end];
                let checksum_start = entry_end;
                if checksum_start + 4 > buffer.len() {
                    break;
                }

                let checksum_bytes = &buffer[checksum_start .. checksum_start + 4];
                let expected_checksum = u32::from_le_bytes(checksum_bytes.try_into().unwrap());

                let mut hasher = Crc32Hasher::new();
                hasher.update(data);
                let actual_checksum = hasher.finalize();

                if actual_checksum == expected_checksum {
                    // Found a valid entry
                    match LogEntry::from_bytes(&buffer[offset .. entry_end + 4]) {
                        Ok(entry) => entries.push(entry),
                        Err(e) => {
                            warn!("Skipping invalid WAL entry: {}", e);
                        },
                    }
                    offset = entry_end + 4;
                    break;
                }
                else {
                    entry_end += 1;
                }
            }

            if entry_end >= buffer.len() {
                break;
            }
        }

        info!("Read {} WAL entries", entries.len());
        Ok(entries)
    }

    /// Perform a checkpoint (truncate the log)
    pub async fn checkpoint(&self) -> Result<()> {
        info!("Performing WAL checkpoint");

        // Close current file
        drop(self.file.write().await);

        // Truncate the file
        tokio::fs::File::create(&self.path).await?;

        // Reopen
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.path)
            .await?;

        *self.file.write().await = file;
        *self.position.write().await = 0;

        info!("WAL checkpoint completed");
        Ok(())
    }

    /// Get the current size of the WAL file
    pub async fn size(&self) -> Result<u64> {
        let metadata = tokio::fs::metadata(&self.path).await?;
        Ok(metadata.len())
    }
}
