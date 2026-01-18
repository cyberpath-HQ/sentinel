//! WAL manager for handling log operations

use std::{fs, path::PathBuf, sync::Arc};

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
    /// Optional compression algorithm for rotated files
    pub compression_algorithm: Option<crate::CompressionAlgorithm>,
    /// Optional maximum number of records per file
    pub max_records_per_file: Option<usize>,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            max_file_size:         Some(10 * 1024 * 1024), // 10MB
            compression_algorithm: Some(crate::CompressionAlgorithm::Zstd),
            max_records_per_file:  Some(1000),
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

        // Check file size limit and rotate if needed
        if let Some(max_size) = self.config.max_file_size {
            let current_size = tokio::fs::metadata(&self.path).await?.len();
            if current_size + entry_size > max_size {
                self.rotate().await?;
            }
        }

        // Check record limit and rotate if needed
        if let Some(max_records) = self.config.max_records_per_file {
            let count = *self.entries_count.lock().await;
            if count >= max_records {
                self.rotate().await?;
            }
        }

        let mut file = self.file.write().await;
        file.write_all(&bytes).await?;
        file.flush().await?;

        *self.entries_count.lock().await += 1;

        debug!("WAL entry written successfully");
        Ok(())
    }

    /// Compress a WAL file using the specified algorithm
    async fn compress_file(path: &PathBuf, alg: crate::CompressionAlgorithm) -> Result<()> {
        let input = tokio::fs::File::open(&path).await?;
        let compressed_path = match alg {
            crate::CompressionAlgorithm::Zstd => path.with_extension("wal.zst"),
            crate::CompressionAlgorithm::Lz4 => path.with_extension("wal.lz4"),
            crate::CompressionAlgorithm::Brotli => path.with_extension("wal.br"),
            crate::CompressionAlgorithm::Deflate => path.with_extension("wal.deflate"),
            crate::CompressionAlgorithm::Gzip => path.with_extension("wal.gz"),
        };
        let output = tokio::fs::File::create(&compressed_path).await?;

        match alg {
            crate::CompressionAlgorithm::Zstd => {
                use async_compression::tokio::bufread::ZstdEncoder;
                let reader = tokio::io::BufReader::new(input);
                let mut encoder = ZstdEncoder::new(reader);
                tokio::io::copy(&mut encoder, &mut tokio::io::BufWriter::new(output)).await?;
            }
            crate::CompressionAlgorithm::Lz4 => {
                use async_compression::tokio::bufread::Lz4Encoder;
                let reader = tokio::io::BufReader::new(input);
                let mut encoder = Lz4Encoder::new(reader);
                tokio::io::copy(&mut encoder, &mut tokio::io::BufWriter::new(output)).await?;
            }
            crate::CompressionAlgorithm::Brotli => {
                use async_compression::tokio::bufread::BrotliEncoder;
                let reader = tokio::io::BufReader::new(input);
                let mut encoder = BrotliEncoder::new(reader);
                tokio::io::copy(&mut encoder, &mut tokio::io::BufWriter::new(output)).await?;
            }
            crate::CompressionAlgorithm::Deflate => {
                use async_compression::tokio::bufread::DeflateEncoder;
                let reader = tokio::io::BufReader::new(input);
                let mut encoder = DeflateEncoder::new(reader);
                tokio::io::copy(&mut encoder, &mut tokio::io::BufWriter::new(output)).await?;
            }
            crate::CompressionAlgorithm::Gzip => {
                use async_compression::tokio::bufread::GzipEncoder;
                let reader = tokio::io::BufReader::new(input);
                let mut encoder = GzipEncoder::new(reader);
                tokio::io::copy(&mut encoder, &mut tokio::io::BufWriter::new(output)).await?;
            }
        }

        // Remove the original file after successful compression
        tokio::fs::remove_file(&path).await?;
        tracing::info!("Compressed WAL file {} to {}", path.display(), compressed_path.display());
        Ok(())
    }

    /// Rotate the WAL file if limits are reached
    async fn rotate(&self) -> Result<()> {
        info!("Rotating WAL file");

        // Close current file
        drop(self.file.write().await);

        // Rename current file to wal.{timestamp}.wal
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_path = self.path.with_extension(format!("{}.wal", timestamp));
        tokio::fs::rename(&self.path, &new_path).await?;

        // If compression is enabled, compress asynchronously
        if let Some(alg) = self.config.compression_algorithm {
            let path = new_path.clone();
            tokio::spawn(async move {
                if let Err(e) = compress_file(&path, alg).await {
                    tracing::error!("Failed to compress WAL file {}: {}", path.display(), e);
                }
            });
        }

        // Create new file
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.path)
            .await?;
        *self.file.write().await = BufWriter::new(file);
        *self.entries_count.lock().await = 0;

        info!("WAL file rotated successfully");
        Ok(())
    }

    /// Parse entries from a buffer
    fn parse_entries_from_buffer(buffer: &[u8]) -> Result<Vec<LogEntry>> {
        let mut entries = Vec::new();
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
        Ok(entries)
    }

    /// Get all WAL files in the directory
    fn get_wal_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        if self.path.exists() {
            files.push(self.path.clone());
        }
        if let Some(parent) = self.path.parent() {
            let dir = fs::read_dir(parent)?;
            for entry in dir {
                let entry = entry?;
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name != self.path.file_name().unwrap().to_str().unwrap() &&
                        file_name.starts_with("wal") &&
                        file_name.ends_with(".wal")
                    {
                        files.push(path);
                    }
                }
            }
        }
        files.sort_by(|a, b| {
            if a == &self.path {
                std::cmp::Ordering::Less
            }
            else if b == &self.path {
                std::cmp::Ordering::Greater
            }
            else {
                a.file_name().cmp(&b.file_name())
            }
        });
        Ok(files)
    }

    /// Read all entries from the WAL (for recovery)
    pub async fn read_all_entries(&self) -> Result<Vec<LogEntry>> {
        info!("Reading all WAL entries for recovery");

        let files = self.get_wal_files()?;
        let mut all_entries = Vec::new();
        for file_path in files {
            let file = File::open(&file_path).await?;
            let mut reader = BufReader::new(file);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer).await?;
            all_entries.extend(Self::parse_entries_from_buffer(&buffer)?);
        }
        *self.entries_count.lock().await = all_entries.len();
        Ok(all_entries)
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
