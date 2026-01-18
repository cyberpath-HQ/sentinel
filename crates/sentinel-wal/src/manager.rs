//! WAL manager for handling log operations

use std::{path::PathBuf, sync::Arc};

use crc32fast::Hasher as Crc32Hasher;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    sync::Mutex,
};
use tracing::{debug, info, warn};
use async_stream::stream;

use crate::{LogEntry, Result};

/// Configuration for WAL manager
#[derive(Debug, Clone)]
pub struct WalConfig {
    /// Optional maximum file size in bytes
    pub max_file_size:        Option<u64>,
    /// Optional compression mode: true for size-optimized, false for performance-optimized
    pub compression_mode:     Option<bool>,
    /// Optional maximum number of records per file
    pub max_records_per_file: Option<usize>,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            max_file_size:        None,
            compression_mode:     None,
            max_records_per_file: None,
        }
    }
}

/// Write-Ahead Log manager
#[derive(Debug)]
pub struct WalManager {
    /// Path to the WAL file
    path:          PathBuf,
    /// Configuration
    config:        WalConfig,
    /// Current WAL file handle
    file:          Arc<tokio::sync::RwLock<BufWriter<File>>>,
    /// Number of entries written
    entries_count: Arc<Mutex<usize>>,
}

impl WalManager {
    /// Create a new WAL manager
    pub async fn new(path: PathBuf, config: WalConfig) -> Result<Self> {
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

        Ok(Self {
            path,
            config,
            file: Arc::new(tokio::sync::RwLock::new(BufWriter::new(file))),
            entries_count: Arc::new(Mutex::new(0)),
        })
    }

    /// Write a log entry to the WAL
    pub async fn write_entry(&self, entry: LogEntry) -> Result<()> {
        debug!("Writing WAL entry: {:?}", entry.entry_type);

        let bytes = entry.to_bytes()?;
        let entry_size = bytes.len() as u64;

        // Check file size limit
        if let Some(max_size) = self.config.max_file_size {
            let current_size = tokio::fs::metadata(&self.path).await?.len();
            if current_size + entry_size > max_size {
                return Err(crate::WalError::FileSizeLimitExceeded.into());
            }
        }

        // Check record limit
        if let Some(max_records) = self.config.max_records_per_file {
            let count = *self.entries_count.lock().await;
            if count >= max_records {
                return Err(crate::WalError::RecordLimitExceeded.into());
            }
        }

        let mut file = self.file.write().await;
        file.write_all(&bytes).await?;
        file.flush().await?;

        *self.entries_count.lock().await += 1;

        debug!("WAL entry written successfully");
        Ok(())
    }

    /// Read all entries from the WAL (for recovery)
    pub async fn read_all_entries(&self) -> Result<Vec<LogEntry>> {
        info!("Reading all WAL entries for recovery");

        let file = File::open(&self.path).await?;
        let mut reader = BufReader::new(file);

        let mut entries = Vec::new();
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).await?;

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
        *self.entries_count.lock().await = entries.len();
        Ok(entries)
    }

    /// Stream all entries from the WAL file
    pub fn stream_entries(&self) -> impl futures::Stream<Item = Result<LogEntry>> + '_ {
        let path = self.path.clone();
        stream! {
            match File::open(&path).await {
                Ok(file) => {
                    let mut reader = BufReader::new(file);
                    let mut buffer = [0u8; 4];
                    loop {
                        // Read length
                        match reader.read_exact(&mut buffer).await {
                            Ok(_) => {
                                let len = u32::from_le_bytes(buffer) as usize;
                                let mut data = vec![0u8; len];
                                match reader.read_exact(&mut data).await {
                                    Ok(_) => {
                                        match LogEntry::from_bytes(&data) {
                                            Ok(entry) => yield Ok(entry),
                                            Err(e) => {
                                                warn!("Skipping invalid WAL entry: {}", e);
                                                // Try to continue, but since length is wrong, may fail
                                            }
                                        }
                                    }
                                    Err(_) => break, // EOF or error
                                }
                            }
                            Err(_) => break, // EOF
                        }
                    }
                }
                Err(e) => yield Err(e.into()),
            }
        }
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

        *self.file.write().await = BufWriter::new(file);
        *self.entries_count.lock().await = 0;

        info!("WAL checkpoint completed");
        Ok(())
    }

    /// Get the current size of the WAL file
    pub async fn size(&self) -> Result<u64> {
        let metadata = tokio::fs::metadata(&self.path).await?;
        Ok(metadata.len())
    }
}
