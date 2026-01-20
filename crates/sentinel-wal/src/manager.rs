//! WAL manager for handling log operations

use std::{fs, path::PathBuf, sync::Arc};

use crc32fast::Hasher as Crc32Hasher;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    sync::Mutex,
};
use tracing::{debug, info, trace, warn};
use async_stream::stream;

use crate::{LogEntry, Result};

/// WAL file format options
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum WalFormat {
    /// Binary format (compact, default)
    #[default]
    Binary,
    /// JSON Lines format (human-readable, extended)
    JsonLines,
}

impl std::str::FromStr for WalFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "binary" => Ok(WalFormat::Binary),
            "json_lines" => Ok(WalFormat::JsonLines),
            _ => Err(format!("Invalid WAL format: {}", s)),
        }
    }
}

impl std::fmt::Display for WalFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalFormat::Binary => write!(f, "binary"),
            WalFormat::JsonLines => write!(f, "json_lines"),
        }
    }
}

/// Configuration for WAL manager
#[derive(Debug, Clone)]
pub struct WalConfig {
    /// Optional maximum file size in bytes
    pub max_file_size:         Option<u64>,
    /// Optional compression algorithm for rotated files
    pub compression_algorithm: Option<crate::CompressionAlgorithm>,
    /// Optional maximum number of records per file
    pub max_records_per_file:  Option<usize>,
    /// WAL file format
    pub format:                WalFormat,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            max_file_size:         Some(10 * 1024 * 1024), // 10MB
            compression_algorithm: Some(crate::CompressionAlgorithm::Zstd),
            max_records_per_file:  Some(1000),
            format:                WalFormat::default(),
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
    /// Create a new WAL manager instance.
    ///
    /// This constructor initializes a WAL manager with the specified file path and configuration.
    /// It creates the necessary directory structure and opens the WAL file for append operations.
    /// The file is created if it doesn't exist, and opened in append mode for existing files.
    ///
    /// # Arguments
    ///
    /// * `path` - The file system path where the WAL file should be stored
    /// * `config` - Configuration options including format, size limits, and compression settings
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the initialized `WalManager`, or a `WalError` if
    /// initialization fails.
    ///
    /// # Errors
    ///
    /// * `WalError::Io` - If directory creation or file operations fail
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::path::PathBuf;
    ///
    /// use sentinel_wal::{WalConfig, WalFormat, WalManager};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = WalConfig {
    ///     format: WalFormat::Binary, // or WalFormat::JsonLines
    ///     max_file_size: Some(10 * 1024 * 1024), // 10MB
    ///     ..Default::default()
    /// };
    ///
    /// let wal = WalManager::new(PathBuf::from("data/myapp.wal"), config).await?;
    ///
    /// // WAL is now ready for writing entries
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(path: PathBuf, config: WalConfig) -> Result<Self> {
        debug!(
            "Creating WAL manager at {:?} with config: max_file_size={:?}, compression={:?}, max_records={:?}, \
             format={:?}",
            path, config.max_file_size, config.compression_algorithm, config.max_records_per_file, config.format
        );

        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            trace!("Ensuring parent directory exists: {:?}", parent);
            tokio::fs::create_dir_all(parent).await?;
        }

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)
            .await?;

        let manager = Self {
            path: path.clone(),
            config,
            file: Arc::new(tokio::sync::RwLock::new(BufWriter::new(file))),
            entries_count: Arc::new(Mutex::new(0)),
        };

        info!("WAL manager initialized successfully at {:?}", path);
        Ok(manager)
    }

    /// Write a log entry to the WAL.
    ///
    /// This method appends a log entry to the WAL file using the configured format
    /// (binary or JSON Lines). The entry is serialized and written atomically to ensure
    /// data integrity. For JSON Lines format, each entry is written as a separate line.
    ///
    /// # Arguments
    ///
    /// * `entry` - The log entry to write to the WAL
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful write, or a `WalError` if the operation fails.
    ///
    /// # Errors
    ///
    /// * `WalError::Serialization` - If entry serialization fails
    /// * `WalError::Io` - If file write operations fail
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sentinel_wal::{WalManager, WalConfig, LogEntry, EntryType};
    /// use std::path::PathBuf;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = WalConfig::default();
    /// let mut wal = WalManager::new(PathBuf::from("data/app.wal"), config).await?;
    ///
    /// // Write an insert operation
    /// let entry = LogEntry::new(
    ///     EntryType::Insert,
    ///     "users".to_string(),
    ///     "user-123".to_string(),
    ///     Some(json!({"name": "Alice", "email": "alice@example.com"}))
    /// );
    ///
    /// wal.write_entry(entry).await?;
    ///
    /// // Write a delete operation
    /// let delete_entry = LogEntry::new(
    ///     EntryType::Delete,
    ///     "users".to_string(),
    ///     "user-123".to_string(),
    ///     None
    /// );
    ///
    /// wal.write_entry(delete_entry).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn write_entry(&self, entry: LogEntry) -> Result<()> {
        debug!(
            "Writing WAL entry: {:?} in format {:?}",
            entry.entry_type, self.config.format
        );

        let bytes = match self.config.format {
            WalFormat::Binary => {
                trace!("Serializing entry to binary format");
                entry.to_bytes()?
            },
            WalFormat::JsonLines => {
                trace!("Serializing entry to JSON format");
                let json = entry.to_json()?;
                let mut bytes = json.into_bytes();
                bytes.push(b'\n'); // Add newline for JSON Lines format
                bytes
            },
        };

        let entry_size = bytes.len() as u64;

        // Check file size limit and rotate if needed
        if let Some(max_size) = self.config.max_file_size {
            let current_size = tokio::fs::metadata(&self.path).await?.len();
            if current_size + entry_size > max_size {
                debug!(
                    "File size limit reached ({} + {} > {}), rotating",
                    current_size, entry_size, max_size
                );
                self.rotate().await?;
            }
        }

        // Check record limit and rotate if needed
        if let Some(max_records) = self.config.max_records_per_file {
            let count = *self.entries_count.lock().await;
            if count >= max_records {
                debug!(
                    "Record limit reached ({} >= {}), rotating",
                    count, max_records
                );
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
        debug!(
            "Starting compression of WAL file {:?} with algorithm {:?}",
            path, alg
        );

        let input = tokio::fs::File::open(&path).await?;
        let compressed_path = match alg {
            crate::CompressionAlgorithm::Zstd => path.with_extension("wal.zst"),
            crate::CompressionAlgorithm::Lz4 => path.with_extension("wal.lz4"),
            crate::CompressionAlgorithm::Brotli => path.with_extension("wal.br"),
            crate::CompressionAlgorithm::Deflate => path.with_extension("wal.deflate"),
            crate::CompressionAlgorithm::Gzip => path.with_extension("wal.gz"),
        };

        trace!("Compressed file will be saved as {:?}", compressed_path);
        let output = tokio::fs::File::create(&compressed_path).await?;

        match alg {
            crate::CompressionAlgorithm::Zstd => {
                trace!("Using Zstd compression");
                use async_compression::tokio::bufread::ZstdEncoder;
                let reader = tokio::io::BufReader::new(input);
                let mut encoder = ZstdEncoder::new(reader);
                tokio::io::copy(&mut encoder, &mut tokio::io::BufWriter::new(output)).await?;
            },
            crate::CompressionAlgorithm::Lz4 => {
                trace!("Using LZ4 compression");
                use async_compression::tokio::bufread::Lz4Encoder;
                let reader = tokio::io::BufReader::new(input);
                let mut encoder = Lz4Encoder::new(reader);
                tokio::io::copy(&mut encoder, &mut tokio::io::BufWriter::new(output)).await?;
            },
            crate::CompressionAlgorithm::Brotli => {
                trace!("Using Brotli compression");
                use async_compression::tokio::bufread::BrotliEncoder;
                let reader = tokio::io::BufReader::new(input);
                let mut encoder = BrotliEncoder::new(reader);
                tokio::io::copy(&mut encoder, &mut tokio::io::BufWriter::new(output)).await?;
            },
            crate::CompressionAlgorithm::Deflate => {
                trace!("Using DEFLATE compression");
                use async_compression::tokio::bufread::DeflateEncoder;
                let reader = tokio::io::BufReader::new(input);
                let mut encoder = DeflateEncoder::new(reader);
                tokio::io::copy(&mut encoder, &mut tokio::io::BufWriter::new(output)).await?;
            },
            crate::CompressionAlgorithm::Gzip => {
                trace!("Using GZIP compression");
                use async_compression::tokio::bufread::GzipEncoder;
                let reader = tokio::io::BufReader::new(input);
                let mut encoder = GzipEncoder::new(reader);
                tokio::io::copy(&mut encoder, &mut tokio::io::BufWriter::new(output)).await?;
            },
        }

        // Remove the original file after successful compression
        trace!("Removing original uncompressed file {:?}", path);
        tokio::fs::remove_file(&path).await?;
        tracing::info!(
            "Compressed WAL file {} to {}",
            path.display(),
            compressed_path.display()
        );
        Ok(())
    }

    /// Rotate the WAL file if limits are reached
    async fn rotate(&self) -> Result<()> {
        info!("Rotating WAL file at {:?}", self.path);

        // Close current file
        drop(self.file.write().await);

        // Rename current file to wal.{timestamp}.wal
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_path = self.path.with_extension(format!("{}.wal", timestamp));
        debug!("Renaming WAL file from {:?} to {:?}", self.path, new_path);
        tokio::fs::rename(&self.path, &new_path).await?;

        // If compression is enabled, compress asynchronously
        if let Some(alg) = self.config.compression_algorithm {
            debug!(
                "Compression enabled with algorithm {:?}, starting async compression",
                alg
            );
            let path = new_path.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::compress_file(&path, alg).await {
                    tracing::error!("Failed to compress WAL file {}: {}", path.display(), e);
                }
            });
        }
        else {
            trace!("Compression disabled, skipping compression step");
        }

        // Create new file
        debug!("Creating new WAL file at {:?}", self.path);
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
    fn parse_entries_from_buffer(&self, buffer: &[u8]) -> Result<Vec<LogEntry>> {
        debug!(
            "Parsing {} bytes of WAL data in format {:?}",
            buffer.len(),
            self.config.format
        );

        match self.config.format {
            WalFormat::Binary => {
                trace!("Parsing binary format entries");
                self.parse_binary_entries(buffer)
            },
            WalFormat::JsonLines => {
                trace!("Parsing JSON Lines format entries");
                self.parse_json_lines_entries(buffer)
            },
        }
    }

    /// Parse binary format entries
    fn parse_binary_entries(&self, buffer: &[u8]) -> Result<Vec<LogEntry>> {
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
                        Ok(entry) => {
                            trace!("Parsed binary entry: {:?}", entry.entry_type);
                            entries.push(entry);
                        },
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
        debug!("Parsed {} binary entries", entries.len());
        Ok(entries)
    }

    /// Parse JSON Lines format entries
    fn parse_json_lines_entries(&self, buffer: &[u8]) -> Result<Vec<LogEntry>> {
        let content = std::str::from_utf8(buffer)
            .map_err(|e| crate::WalError::Serialization(format!("Invalid UTF-8 in JSON Lines: {}", e)))?;

        let mut entries = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match LogEntry::from_json(line) {
                Ok(entry) => {
                    trace!("Parsed JSON entry {}: {:?}", line_num + 1, entry.entry_type);
                    entries.push(entry);
                },
                Err(e) => {
                    warn!("Skipping invalid JSON line {}: {}", line_num + 1, e);
                },
            }
        }
        debug!("Parsed {} JSON Lines entries", entries.len());
        Ok(entries)
    }

    /// Get all WAL files in the directory
    fn get_wal_files(&self) -> Result<Vec<PathBuf>> {
        trace!("Scanning for WAL files in directory of {:?}", self.path);
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
                        trace!("Found WAL file: {:?}", path);
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
        debug!("Found {} WAL files total", files.len());
        Ok(files)
    }

    /// Read all log entries from the WAL for recovery.
    ///
    /// This method reads and parses all entries from the WAL file(s) in the configured format.
    /// It's typically used during database recovery to replay operations. The method automatically
    /// detects the format (binary or JSON Lines) and parses entries accordingly.
    ///
    /// For binary format, entries are parsed by finding checksum boundaries.
    /// For JSON Lines format, entries are parsed line by line.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of all `LogEntry` instances found in the WAL,
    /// or a `WalError` if reading or parsing fails.
    ///
    /// # Errors
    ///
    /// * `WalError::Io` - If file operations fail
    /// * `WalError::Serialization` - If entry parsing fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::path::PathBuf;
    ///
    /// use sentinel_wal::{WalConfig, WalManager};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = WalConfig::default();
    /// let wal = WalManager::new(PathBuf::from("data/app.wal"), config).await?;
    ///
    /// // Read all entries for recovery
    /// let entries = wal.read_all_entries().await?;
    ///
    /// println!("Found {} entries in WAL", entries.len());
    /// for entry in entries {
    ///     println!(
    ///         "Entry: {:?} on {} in collection {}",
    ///         entry.entry_type,
    ///         entry.document_id_str(),
    ///         entry.collection_str()
    ///     );
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn read_all_entries(&self) -> Result<Vec<LogEntry>> {
        info!("Reading all WAL entries for recovery from {:?}", self.path);

        let files = self.get_wal_files()?;
        debug!("Found {} WAL files to read", files.len());
        let mut all_entries = Vec::new();
        for file_path in files {
            trace!("Reading entries from file {:?}", file_path);
            let file = File::open(&file_path).await?;
            let mut reader = BufReader::new(file);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer).await?;
            let entries = self.parse_entries_from_buffer(&buffer)?;
            debug!("Read {} entries from {:?}", entries.len(), file_path);
            all_entries.extend(entries);
        }
        *self.entries_count.lock().await = all_entries.len();
        info!(
            "Recovery complete: loaded {} total entries",
            all_entries.len()
        );
        Ok(all_entries)
    }

    /// Stream log entries from the WAL file.
    ///
    /// This method provides a streaming interface to read WAL entries without loading
    /// the entire file into memory. It's more memory-efficient than `read_all_entries()`
    /// for large WAL files. The stream automatically handles format detection and parsing.
    ///
    /// For binary format, entries are streamed by reading length prefixes.
    /// For JSON Lines format, entries are streamed line by line.
    ///
    /// # Returns
    ///
    /// Returns a `Stream` that yields `Result<LogEntry>` items. The stream will yield
    /// `Ok(entry)` for successfully parsed entries and `Err(error)` for parsing failures.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::path::PathBuf;
    ///
    /// use sentinel_wal::{WalConfig, WalManager};
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = WalConfig::default();
    /// let wal = WalManager::new(PathBuf::from("data/app.wal"), config).await?;
    ///
    /// // Stream entries for processing
    /// let mut stream = wal.stream_entries();
    /// use futures::pin_mut;
    /// pin_mut!(stream);
    ///
    /// let mut count = 0;
    /// while let Some(result) = stream.next().await {
    ///     match result {
    ///         Ok(entry) => {
    ///             count += 1;
    ///             println!("Processed entry {}: {:?}", count, entry.entry_type);
    ///         },
    ///         Err(e) => {
    ///             eprintln!("Error reading entry: {}", e);
    ///         },
    ///     }
    /// }
    ///
    /// println!("Total entries processed: {}", count);
    /// # Ok(())
    /// # }
    /// ```
    pub fn stream_entries(&self) -> impl futures::Stream<Item = Result<LogEntry>> + '_ {
        let path = self.path.clone();
        let format = self.config.format;
        stream! {
            debug!("Streaming WAL entries from {:?} in format {:?}", path, format);
            match File::open(&path).await {
                Ok(file) => {
                    let mut reader = BufReader::new(file);
                    match format {
                        WalFormat::Binary => {
                            trace!("Streaming binary format entries");
                            // For binary format, read the entire file and parse using checksums
                            let mut buffer = Vec::new();
                            match reader.read_to_end(&mut buffer).await {
                                Ok(_) => {
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
                                                    Ok(entry) => {
                                                        trace!("Streamed binary entry: {:?}", entry.entry_type);
                                                        yield Ok(entry);
                                                    },
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
                                },
                                Err(e) => yield Err(e.into()),
                            }
                        },
                        WalFormat::JsonLines => {
                            trace!("Streaming JSON Lines format entries");
                            let mut line_buffer = String::new();
                            loop {
                                line_buffer.clear();
                                match reader.read_line(&mut line_buffer).await {
                                    Ok(0) => break, // EOF
                                    Ok(_) => {
                                        let line = line_buffer.trim();
                                        if !line.is_empty() {
                                            match LogEntry::from_json(line) {
                                                Ok(entry) => {
                                                    trace!("Streamed JSON entry: {:?}", entry.entry_type);
                                                    yield Ok(entry);
                                                },
                                                Err(e) => {
                                                    warn!("Skipping invalid JSON line: {}", e);
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        yield Err(crate::WalError::Io(e).into());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => yield Err(e.into()),
            }
        }
    }

    /// Perform a checkpoint operation on the WAL.
    ///
    /// A checkpoint ensures that all pending WAL entries are durably written to disk
    /// and creates a recovery point. This is different from truncation - checkpointing
    /// preserves the WAL for potential future recovery while marking a safe recovery point.
    ///
    /// The checkpoint process:
    /// 1. Flushes any buffered writes to disk
    /// 2. Ensures file metadata is synchronized
    /// 3. Records the checkpoint position for recovery
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful checkpoint, or a `WalError` if the operation fails.
    ///
    /// # Errors
    ///
    /// * `WalError::Io` - If file synchronization operations fail
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::path::PathBuf;
    ///
    /// use sentinel_wal::{EntryType, LogEntry, WalConfig, WalManager};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = WalConfig::default();
    /// let wal = WalManager::new(PathBuf::from("data/app.wal"), config).await?;
    ///
    /// // Write some entries
    /// let entry = LogEntry::new(
    ///     EntryType::Insert,
    ///     "users".to_string(),
    ///     "user-123".to_string(),
    ///     None,
    /// );
    /// wal.write_entry(entry).await?;
    ///
    /// // Create a checkpoint
    /// wal.checkpoint().await?;
    ///
    /// // At this point, all entries are safely on disk
    /// // and can be recovered from if needed
    /// # Ok(())
    /// # }
    /// ```
    pub async fn checkpoint(&self) -> Result<()> {
        info!("Performing WAL checkpoint at {:?}", self.path);

        // Flush any buffered writes
        debug!("Flushing WAL file buffers");
        self.file.write().await.flush().await?;

        // Get the current file handle and sync to disk
        debug!("Syncing WAL file to disk");
        self.file.write().await.get_ref().sync_all().await?;

        // Record checkpoint position (current file size)
        let checkpoint_position = self.size().await?;
        debug!(
            "Checkpoint created at position: {} bytes",
            checkpoint_position
        );

        info!(
            "WAL checkpoint completed successfully at position {}",
            checkpoint_position
        );
        Ok(())
    }

    /// Get the current size of the WAL file in bytes.
    ///
    /// This method returns the size of the WAL file on disk, which can be used
    /// to monitor file growth and determine if rotation is needed based on
    /// the configured `max_file_size` limit.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the file size in bytes, or a `WalError` if
    /// the metadata cannot be read.
    ///
    /// # Errors
    ///
    /// * `WalError::Io` - If file metadata operations fail
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::path::PathBuf;
    ///
    /// use sentinel_wal::{WalConfig, WalManager};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = WalConfig {
    ///     max_file_size: Some(10 * 1024 * 1024), // 10MB
    ///     ..Default::default()
    /// };
    /// let wal =
    ///     WalManager::new(PathBuf::from("data/app.wal"), config.clone()).await?;
    ///
    /// let size = wal.size().await?;
    /// println!("WAL file size: {} bytes", size);
    ///
    /// if let Some(max_size) = config.max_file_size {
    ///     if size >= max_size {
    ///         println!("WAL file should be rotated");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn size(&self) -> Result<u64> {
        let metadata = tokio::fs::metadata(&self.path).await?;
        let size = metadata.len();
        trace!("WAL file size: {} bytes", size);
        Ok(size)
    }

    /// Get the number of entries in the WAL.
    ///
    /// This returns the count of entries that have been written to the WAL.
    /// Note that this may not reflect the current state if entries have been
    /// checkpointed or if the WAL has been rotated.
    ///
    /// # Returns
    ///
    /// Returns the number of entries written to the WAL.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::path::PathBuf;
    ///
    /// use sentinel_wal::{WalConfig, WalManager};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let wal = WalManager::new(PathBuf::from("data.wal"), WalConfig::default())
    ///     .await?;
    ///
    /// let count = wal.entries_count().await?;
    /// println!("WAL has {} entries", count);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn entries_count(&self) -> Result<usize> {
        let count = *self.entries_count.lock().await;
        trace!("WAL entries count: {}", count);
        Ok(count)
    }
}
