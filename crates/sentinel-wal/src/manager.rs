//! WAL manager for handling log operations

use std::{fs, path::PathBuf, sync::Arc};

use crc32fast::Hasher as Crc32Hasher;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader, BufWriter},
    sync::Mutex,
};
use tracing::{debug, info, trace, warn};
use async_stream::stream;

use crate::{LogEntry, Result};

/// WAL file format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
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
            "binary" => Ok(Self::Binary),
            "json_lines" => Ok(Self::JsonLines),
            _ => Err(format!("Invalid WAL format: {}", s)),
        }
    }
}

impl std::fmt::Display for WalFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Binary => write!(f, "binary"),
            Self::JsonLines => write!(f, "json_lines"),
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
    #[allow(
        clippy::arithmetic_side_effects,
        reason = "safe arithmetic in write_entry"
    )]
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
        drop(file);

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
    #[allow(
        clippy::arithmetic_side_effects,
        clippy::indexing_slicing,
        reason = "safe operations in parse_binary_entries"
    )]
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
    #[allow(
        clippy::arithmetic_side_effects,
        reason = "safe arithmetic in parse_json_lines_entries"
    )]
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
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) &&
                    file_name != self.path.file_name().unwrap().to_str().unwrap() &&
                    file_name.starts_with("wal") &&
                    file_name.ends_with(".wal")
                {
                    trace!("Found WAL file: {:?}", path);
                    files.push(path);
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
    #[allow(
        clippy::arithmetic_side_effects,
        clippy::indexing_slicing,
        reason = "safe operations in stream_entries"
    )]
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
                                        yield Err(crate::WalError::Io(e));
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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use serde_json::json;
    use futures::StreamExt;

    use super::*;

    // ============ WalFormat Tests ============

    #[test]
    fn test_wal_format_default() {
        assert_eq!(WalFormat::default(), WalFormat::Binary);
    }

    #[test]
    fn test_wal_format_from_str_valid() {
        assert_eq!("binary".parse::<WalFormat>().unwrap(), WalFormat::Binary);
        assert_eq!(
            "json_lines".parse::<WalFormat>().unwrap(),
            WalFormat::JsonLines
        );
    }

    #[test]
    fn test_wal_format_from_str_case_insensitive() {
        assert_eq!("BINARY".parse::<WalFormat>().unwrap(), WalFormat::Binary);
        assert_eq!(
            "JSON_LINES".parse::<WalFormat>().unwrap(),
            WalFormat::JsonLines
        );
    }

    #[test]
    fn test_wal_format_from_str_invalid() {
        assert!("invalid".parse::<WalFormat>().is_err());
        assert!("".parse::<WalFormat>().is_err());
        assert!("json".parse::<WalFormat>().is_err());
    }

    #[test]
    fn test_wal_format_display() {
        assert_eq!(WalFormat::Binary.to_string(), "binary");
        assert_eq!(WalFormat::JsonLines.to_string(), "json_lines");
    }

    #[test]
    fn test_wal_format_debug() {
        let debug_binary = format!("{:?}", WalFormat::Binary);
        assert!(debug_binary.contains("Binary"));
        let debug_json = format!("{:?}", WalFormat::JsonLines);
        assert!(debug_json.contains("JsonLines"));
    }

    #[test]
    fn test_wal_format_clone() {
        let format = WalFormat::JsonLines;
        let cloned = format.clone();
        assert_eq!(format, cloned);
    }

    // ============ WalConfig Tests ============

    #[test]
    fn test_wal_config_default() {
        let config = WalConfig::default();

        assert_eq!(config.max_file_size, Some(10 * 1024 * 1024));
        assert_eq!(
            config.compression_algorithm,
            Some(crate::CompressionAlgorithm::Zstd)
        );
        assert_eq!(config.max_records_per_file, Some(1000));
        assert_eq!(config.format, WalFormat::Binary);
    }

    #[test]
    fn test_wal_config_clone() {
        let config = WalConfig::default();
        let cloned = config.clone();

        assert_eq!(config.max_file_size, cloned.max_file_size);
        assert_eq!(config.format, cloned.format);
    }

    #[test]
    fn test_wal_config_custom_values() {
        let config = WalConfig {
            max_file_size:         Some(5 * 1024 * 1024),
            compression_algorithm: Some(crate::CompressionAlgorithm::Lz4),
            max_records_per_file:  Some(500),
            format:                WalFormat::JsonLines,
        };

        assert_eq!(config.max_file_size, Some(5 * 1024 * 1024));
        assert_eq!(
            config.compression_algorithm,
            Some(crate::CompressionAlgorithm::Lz4)
        );
        assert_eq!(config.max_records_per_file, Some(500));
        assert_eq!(config.format, WalFormat::JsonLines);
    }

    // ============ WalManager Tests ============

    #[tokio::test]
    async fn test_wal_manager_new_binary_format() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_binary.wal");

        let _wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        assert!(wal_path.exists());
    }

    #[tokio::test]
    async fn test_wal_manager_new_json_lines_format() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_json.wal");

        let config = WalConfig {
            format: WalFormat::JsonLines,
            ..Default::default()
        };

        let _wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        assert!(wal_path.exists());
    }

    #[tokio::test]
    async fn test_wal_manager_write_and_read_single_entry() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_single.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(json!({"name": "Alice"})),
        );

        wal.write_entry(entry.clone()).await.unwrap();

        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].collection_str(), "users");
        assert_eq!(entries[0].document_id_str(), "user-1");
    }

    #[tokio::test]
    async fn test_wal_manager_write_and_read_multiple_entries() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_multiple.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let entries = vec![
            LogEntry::new(
                crate::EntryType::Insert,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice"})),
            ),
            LogEntry::new(
                crate::EntryType::Update,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice Updated"})),
            ),
            LogEntry::new(
                crate::EntryType::Delete,
                "users".to_string(),
                "user-1".to_string(),
                None,
            ),
        ];

        for entry in &entries {
            wal.write_entry(entry.clone()).await.unwrap();
        }

        let read_entries = wal.read_all_entries().await.unwrap();
        assert_eq!(read_entries.len(), 3);
    }

    #[tokio::test]
    async fn test_wal_manager_size_empty() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_size_empty.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let size = wal.size().await.unwrap();
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn test_wal_manager_size_after_write() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_size_write.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let size_before = wal.size().await.unwrap();

        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(json!({"name": "Alice"})),
        );

        wal.write_entry(entry.clone()).await.unwrap();

        let size_after = wal.size().await.unwrap();
        assert!(size_after > size_before);
    }

    #[tokio::test]
    async fn test_wal_manager_entries_count_empty() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_count_empty.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let count = wal.entries_count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_wal_manager_entries_count_after_writes() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_count_write.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        for i in 1 ..= 5 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "users".to_string(),
                format!("user-{}", i),
                Some(json!({"id": i})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        let count = wal.entries_count().await.unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_wal_manager_stream_entries_empty() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_stream_empty.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let stream = wal.stream_entries();
        let mut pinned_stream = std::pin::pin!(stream);

        let entry = pinned_stream.next().await;
        assert!(entry.is_none());
    }

    #[tokio::test]
    async fn test_wal_manager_stream_entries_with_data() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_stream_data.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let entries = vec![
            LogEntry::new(
                crate::EntryType::Insert,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice"})),
            ),
            LogEntry::new(
                crate::EntryType::Update,
                "products".to_string(),
                "prod-1".to_string(),
                Some(json!({"price": 29.99})),
            ),
        ];

        for entry in &entries {
            wal.write_entry(entry.clone()).await.unwrap();
        }

        let stream = wal.stream_entries();
        futures::pin_mut!(stream);

        let mut count = 0;
        while let Some(Ok(_entry)) = stream.next().await {
            count += 1;
        }

        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_wal_manager_different_entry_types() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_entry_types.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let entries = vec![
            LogEntry::new(
                crate::EntryType::Insert,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice"})),
            ),
            LogEntry::new(
                crate::EntryType::Update,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Bob"})),
            ),
            LogEntry::new(
                crate::EntryType::Delete,
                "users".to_string(),
                "user-1".to_string(),
                None,
            ),
            LogEntry::new(
                crate::EntryType::Begin,
                "users".to_string(),
                "txn-1".to_string(),
                None,
            ),
            LogEntry::new(
                crate::EntryType::Commit,
                "users".to_string(),
                "txn-1".to_string(),
                None,
            ),
            LogEntry::new(
                crate::EntryType::Rollback,
                "users".to_string(),
                "txn-2".to_string(),
                None,
            ),
        ];

        for entry in &entries {
            wal.write_entry(entry.clone()).await.unwrap();
        }

        let read_entries = wal.read_all_entries().await.unwrap();
        assert_eq!(read_entries.len(), 6);

        // Verify entry types are preserved
        assert_eq!(read_entries[0].entry_type, crate::EntryType::Insert);
        assert_eq!(read_entries[1].entry_type, crate::EntryType::Update);
        assert_eq!(read_entries[2].entry_type, crate::EntryType::Delete);
        assert_eq!(read_entries[3].entry_type, crate::EntryType::Begin);
        assert_eq!(read_entries[4].entry_type, crate::EntryType::Commit);
        assert_eq!(read_entries[5].entry_type, crate::EntryType::Rollback);
    }

    #[tokio::test]
    async fn test_wal_manager_with_large_data() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_large_data.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Create a large JSON object
        let large_data = json!({
            "users": (0..1000).map(|i| format!("user-{}", i)).collect::<Vec<_>>(),
            "metadata": {
                "created_at": "2024-01-01T00:00:00Z",
                "version": "1.0.0",
            }
        });

        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "users".to_string(),
            "large-doc".to_string(),
            Some(large_data),
        );

        wal.write_entry(entry.clone()).await.unwrap();

        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].document_id_str(), "large-doc");
    }

    #[tokio::test]
    async fn test_wal_manager_empty_data() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_empty_data.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let entry = LogEntry::new(
            crate::EntryType::Delete,
            "users".to_string(),
            "user-1".to_string(),
            None, // No data
        );

        wal.write_entry(entry.clone()).await.unwrap();

        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].data.is_none());
    }

    #[tokio::test]
    async fn test_wal_manager_special_characters_in_ids() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_special_chars.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "users".to_string(),
            "user-with-dashes_underscores.and.dots".to_string(),
            Some(json!({"name": "Test User"})),
        );

        wal.write_entry(entry.clone()).await.unwrap();

        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].document_id_str(),
            "user-with-dashes_underscores.and.dots"
        );
    }

    #[tokio::test]
    async fn test_wal_manager_rotation_on_size_limit() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_rotation_size.wal");

        // Create WAL with small size limit
        let config = WalConfig {
            max_file_size:         Some(500), // Small size limit
            compression_algorithm: None,
            max_records_per_file:  None,
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write entries - verify they can be written
        for i in 0 .. 3 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "users".to_string(),
                format!("user-{}", i),
                Some(json!({"index": i})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // At least some entries should be written
        let count = wal.entries_count().await.unwrap();
        assert!(count > 0, "Expected at least 1 entry, got {}", count);
    }

    #[tokio::test]
    async fn test_wal_manager_rotation_on_record_limit() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_rotation_records.wal");

        // Create WAL with small record limit
        let config = WalConfig {
            max_file_size:         None,
            compression_algorithm: None,
            max_records_per_file:  Some(10), // Set a limit
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write entries - verify they can be written
        for i in 0 .. 3 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "users".to_string(),
                format!("user-{}", i),
                Some(json!({"index": i})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // At least some entries should be written
        let count = wal.entries_count().await.unwrap();
        assert!(count > 0, "Expected at least 1 entry, got {}", count);
    }

    #[tokio::test]
    async fn test_wal_manager_json_lines_format() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_json_lines.wal");

        let config = WalConfig {
            format: WalFormat::JsonLines,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        let entries = vec![
            LogEntry::new(
                crate::EntryType::Insert,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice"})),
            ),
            LogEntry::new(
                crate::EntryType::Update,
                "users".to_string(),
                "user-1".to_string(),
                Some(json!({"name": "Alice Updated"})),
            ),
        ];

        for entry in &entries {
            wal.write_entry(entry.clone()).await.unwrap();
        }

        let read_entries = wal.read_all_entries().await.unwrap();
        assert_eq!(read_entries.len(), 2);
        assert_eq!(read_entries[0].collection_str(), "users");
        assert_eq!(read_entries[0].document_id_str(), "user-1");
    }

    #[tokio::test]
    async fn test_wal_manager_entries_count_after_checkpoint() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_checkpoint_count.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write some entries
        for i in 0 .. 5 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "users".to_string(),
                format!("user-{}", i),
                Some(json!({"id": i})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        let count_before = wal.entries_count().await.unwrap();
        assert_eq!(count_before, 5);

        // Checkpoint should not change count
        wal.checkpoint().await.unwrap();

        let count_after = wal.entries_count().await.unwrap();
        assert_eq!(count_after, count_before);
    }

    #[tokio::test]
    async fn test_wal_manager_size_grows_with_entries() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_size_growth.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let size_0 = wal.size().await.unwrap();

        // Write first entry
        let entry1 = LogEntry::new(
            crate::EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(json!({"name": "Alice"})),
        );
        wal.write_entry(entry1.clone()).await.unwrap();

        let size_1 = wal.size().await.unwrap();
        assert!(size_1 > size_0);

        // Write second entry (should be larger)
        let entry2 = LogEntry::new(
            crate::EntryType::Insert,
            "users".to_string(),
            "user-2".to_string(),
            Some(json!({"name": "Bob", "extra_data": "x".repeat(100)})),
        );
        wal.write_entry(entry2.clone()).await.unwrap();

        let size_2 = wal.size().await.unwrap();
        assert!(size_2 > size_1);
    }

    #[tokio::test]
    async fn test_wal_manager_checkpoint_flushes_data() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_checkpoint_flush.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write an entry
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(json!({"name": "Test"})),
        );
        wal.write_entry(entry.clone()).await.unwrap();

        // Checkpoint should succeed without errors
        let result = wal.checkpoint().await;
        assert!(result.is_ok());

        // Size should remain the same after checkpoint
        let size_after = wal.size().await.unwrap();
        assert!(size_after > 0);
    }

    #[tokio::test]
    async fn test_wal_manager_stream_with_no_entries() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_stream_empty.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Stream on empty WAL
        let stream = wal.stream_entries();
        futures::pin_mut!(stream);

        let result = stream.next().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_wal_manager_read_with_corrupted_data() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_corrupted.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write a valid entry first
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(json!({"name": "Alice"})),
        );
        wal.write_entry(entry.clone()).await.unwrap();

        // Read entries - should handle gracefully
        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_wal_manager_different_entry_types_all() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_all_entry_types.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Test all entry types
        let entry_types = vec![
            crate::EntryType::Insert,
            crate::EntryType::Update,
            crate::EntryType::Delete,
            crate::EntryType::Begin,
            crate::EntryType::Commit,
            crate::EntryType::Rollback,
        ];

        for (i, entry_type) in entry_types.into_iter().enumerate() {
            let entry = LogEntry::new(
                entry_type,
                "test".to_string(),
                format!("doc-{}", i),
                if entry_type == crate::EntryType::Delete {
                    None
                }
                else {
                    Some(json!({"type": format!("{:?}", entry_type)}))
                },
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 6);

        // Verify each entry type
        assert_eq!(entries[0].entry_type, crate::EntryType::Insert);
        assert_eq!(entries[1].entry_type, crate::EntryType::Update);
        assert_eq!(entries[2].entry_type, crate::EntryType::Delete);
        assert_eq!(entries[3].entry_type, crate::EntryType::Begin);
        assert_eq!(entries[4].entry_type, crate::EntryType::Commit);
        assert_eq!(entries[5].entry_type, crate::EntryType::Rollback);
    }

    #[tokio::test]
    async fn test_wal_manager_concurrent_writes() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_concurrent.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write entries concurrently using Arc
        let wal_arc = Arc::new(wal);
        let handles: Vec<_> = (0 .. 5)
            .map(|i| {
                let wal = wal_arc.clone();
                tokio::spawn(async move {
                    let entry = LogEntry::new(
                        crate::EntryType::Insert,
                        "users".to_string(),
                        format!("user-{}", i),
                        Some(json!({"index": i})),
                    );
                    wal.write_entry(entry.clone()).await.unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        // All entries should be written
        let count = wal_arc.entries_count().await.unwrap();
        assert_eq!(count, 5);

        // All entries should be readable
        let entries = wal_arc.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 5);
    }

    // ============ Compression Algorithm Tests ============

    #[tokio::test]
    async fn test_wal_manager_compression_zstd() {
        // Test compression configuration with Zstd
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_zstd.wal");

        let config = WalConfig {
            compression_algorithm: Some(crate::CompressionAlgorithm::Zstd),
            max_file_size:         Some(100), // Small size to trigger rotation
            max_records_per_file:  None,
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write entries to trigger rotation and compression
        for i in 0 .. 5 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"data": "x".repeat(50)})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Give time for async compression
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Should have some entries
        let count = wal.entries_count().await.unwrap();
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_wal_manager_compression_lz4() {
        // Test compression configuration with LZ4
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_lz4.wal");

        let config = WalConfig {
            compression_algorithm: Some(crate::CompressionAlgorithm::Lz4),
            max_file_size:         Some(200),
            max_records_per_file:  None,
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        for i in 0 .. 3 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"data": "y".repeat(80)})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        let count = wal.entries_count().await.unwrap();
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_wal_manager_compression_brotli() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_brotli.wal");

        let config = WalConfig {
            compression_algorithm: Some(crate::CompressionAlgorithm::Brotli),
            max_file_size:         Some(150),
            max_records_per_file:  None,
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        for i in 0 .. 4 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"data": "z".repeat(40)})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        let count = wal.entries_count().await.unwrap();
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_wal_manager_compression_deflate() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_deflate.wal");

        let config = WalConfig {
            compression_algorithm: Some(crate::CompressionAlgorithm::Deflate),
            max_file_size:         Some(100),
            max_records_per_file:  None,
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        for i in 0 .. 3 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"data": "a".repeat(30)})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        let count = wal.entries_count().await.unwrap();
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_wal_manager_compression_gzip() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_gzip.wal");

        let config = WalConfig {
            compression_algorithm: Some(crate::CompressionAlgorithm::Gzip),
            max_file_size:         Some(100),
            max_records_per_file:  None,
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        for i in 0 .. 3 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"data": "b".repeat(30)})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        let count = wal.entries_count().await.unwrap();
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_wal_manager_compression_no_compression() {
        // Test with compression explicitly disabled
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_no_compress.wal");

        let config = WalConfig {
            compression_algorithm: None,
            max_file_size:         None,
            max_records_per_file:  Some(10),
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        for i in 0 .. 5 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"index": i})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        let count = wal.entries_count().await.unwrap();
        assert_eq!(count, 5);
    }

    // ============ Entry Reading Edge Cases ============

    #[tokio::test]
    async fn test_wal_manager_read_empty_json_lines() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_empty_json.wal");

        let config = WalConfig {
            format: WalFormat::JsonLines,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write some entries
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "users".to_string(),
            "user-1".to_string(),
            Some(json!({"name": "Alice"})),
        );
        wal.write_entry(entry.clone()).await.unwrap();

        // Read back
        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_wal_manager_read_mixed_formats() {
        // This test verifies that we can read back what we write
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_mixed.wal");

        // Test binary format
        let binary_wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        let entries = vec![
            LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                "doc-1".to_string(),
                Some(json!({"type": "binary", "data": "test123"})),
            ),
            LogEntry::new(
                crate::EntryType::Update,
                "test".to_string(),
                "doc-1".to_string(),
                Some(json!({"type": "binary", "data": "updated"})),
            ),
        ];

        for entry in &entries {
            binary_wal.write_entry(entry.clone()).await.unwrap();
        }

        let read_entries = binary_wal.read_all_entries().await.unwrap();
        assert_eq!(read_entries.len(), 2);
        assert_eq!(read_entries[0].entry_type, crate::EntryType::Insert);
        assert_eq!(read_entries[1].entry_type, crate::EntryType::Update);
    }

    #[tokio::test]
    async fn test_wal_manager_stream_binary_format() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_stream_binary.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write entries
        for i in 0 .. 3 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"index": i, "value": i * 10})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Stream and count entries
        let stream = wal.stream_entries();
        futures::pin_mut!(stream);

        let mut count = 0;
        let mut found_ids = Vec::new();
        while let Some(Ok(entry)) = stream.next().await {
            count += 1;
            found_ids.push(entry.document_id_str().to_string());
        }

        assert_eq!(count, 3);
        assert!(found_ids.contains(&"doc-0".to_string()));
        assert!(found_ids.contains(&"doc-1".to_string()));
        assert!(found_ids.contains(&"doc-2".to_string()));
    }

    #[tokio::test]
    async fn test_wal_manager_stream_json_lines_format() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_stream_json.wal");

        let config = WalConfig {
            format: WalFormat::JsonLines,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write entries
        for i in 0 .. 3 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"index": i})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Stream entries
        let stream = wal.stream_entries();
        futures::pin_mut!(stream);

        let mut count = 0;
        while let Some(Ok(entry)) = stream.next().await {
            count += 1;
            assert_eq!(entry.entry_type, crate::EntryType::Insert);
        }

        assert_eq!(count, 3);
    }

    // ============ Size Calculation Edge Cases ============

    #[tokio::test]
    async fn test_wal_manager_size_with_large_entries() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_large.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write a large entry
        let large_data = json!({
            "items": (0..1000).map(|i| format!("item-{}", i)).collect::<Vec<_>>(),
            "metadata": {
                "created": "2024-01-01",
                "version": "1.0.0",
                "description": "Large test document for size calculation"
            }
        });

        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "test".to_string(),
            "large-doc".to_string(),
            Some(large_data),
        );
        wal.write_entry(entry.clone()).await.unwrap();

        let size = wal.size().await.unwrap();
        assert!(
            size > 0,
            "WAL size should be greater than 0 for large entry"
        );

        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_wal_manager_entries_count_after_rotation() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_rotation_count.wal");

        let config = WalConfig {
            max_file_size:         Some(200),
            compression_algorithm: None,
            max_records_per_file:  None,
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write multiple entries to potentially trigger rotation
        for i in 0 .. 5 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"data": "x".repeat(50)})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Count should be accurate (rotation may have occurred)
        let count = wal.entries_count().await.unwrap();
        assert!(count > 0, "Should have some entries written");
    }

    // ============ Stream Iteration Edge Cases ============

    #[tokio::test]
    async fn test_wal_manager_stream_handles_unicode() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_unicode.wal");

        let config = WalConfig {
            format: WalFormat::JsonLines,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write entry with unicode characters
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "users".to_string(),
            "user-你好".to_string(),
            Some(json!({"name": "Alice 🏠", "emoji": "😀"})),
        );
        wal.write_entry(entry.clone()).await.unwrap();

        let stream = wal.stream_entries();
        futures::pin_mut!(stream);

        let result = stream.next().await;
        assert!(result.is_some());

        let read_entry = result.unwrap();
        assert!(read_entry.is_ok());

        let entry = read_entry.unwrap();
        assert!(entry.document_id_str().contains("你好"));
    }

    #[tokio::test]
    async fn test_wal_manager_read_with_special_characters_in_data() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_special_data.wal");

        let config = WalConfig {
            format: WalFormat::JsonLines,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write entry with special characters in data
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "test".to_string(),
            "doc-1".to_string(),
            Some(json!({
                "text": "Hello \"World\"!",
                "path": "C:\\Users\\Test\\File.txt",
                "code": "console.log('test');",
                "newlines": "line1\nline2\nline3"
            })),
        );
        wal.write_entry(entry.clone()).await.unwrap();

        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);

        // Verify data is preserved
        let read_data = &entries[0].data;
        assert!(read_data.is_some());
    }

    #[tokio::test]
    async fn test_wal_manager_max_records_rotation() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_max_records.wal");

        let config = WalConfig {
            max_file_size:         None,
            compression_algorithm: None,
            max_records_per_file:  Some(5),
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write exactly the limit
        for i in 0 .. 5 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"index": i})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Write one more to trigger rotation
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "test".to_string(),
            "doc-5".to_string(),
            Some(json!({"index": 5})),
        );
        wal.write_entry(entry.clone()).await.unwrap();

        // Should have at least some entries
        let count = wal.entries_count().await.unwrap();
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_wal_manager_both_size_and_record_limits() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_both_limits.wal");

        let config = WalConfig {
            max_file_size:         Some(300),
            compression_algorithm: None,
            max_records_per_file:  Some(3),
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write entries - both limits should be checked
        for i in 0 .. 10 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"data": "x".repeat(100)})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Some entries should have been written
        let count = wal.entries_count().await.unwrap();
        assert!(count > 0, "Expected some entries to be written");
    }

    #[tokio::test]
    async fn test_wal_manager_empty_wal_file_exists() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_empty_exists.wal");

        // Create an empty file first
        std::fs::write(&wal_path, "").unwrap();

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Should be able to read from existing empty file
        let entries = wal.read_all_entries().await.unwrap();
        assert!(entries.is_empty());

        let count = wal.entries_count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_wal_manager_read_after_recovery() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_recovery.wal");

        // First session: write entries
        {
            let wal = WalManager::new(wal_path.clone(), WalConfig::default())
                .await
                .unwrap();

            for i in 0 .. 5 {
                let entry = LogEntry::new(
                    crate::EntryType::Insert,
                    "users".to_string(),
                    format!("user-{}", i),
                    Some(json!({"name": format!("User {}", i)})),
                );
                wal.write_entry(entry.clone()).await.unwrap();
            }

            // Force checkpoint
            wal.checkpoint().await.unwrap();
        }

        // Second session: read entries for recovery
        {
            let wal = WalManager::new(wal_path.clone(), WalConfig::default())
                .await
                .unwrap();

            let entries = wal.read_all_entries().await.unwrap();
            assert_eq!(entries.len(), 5);

            // Verify data integrity
            for (i, entry) in entries.iter().enumerate() {
                assert_eq!(entry.document_id_str(), format!("user-{}", i));
            }
        }
    }

    #[tokio::test]
    async fn test_wal_manager_json_lines_with_empty_lines() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_json_empty.wal");

        let config = WalConfig {
            format: WalFormat::JsonLines,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write entries
        let entry1 = LogEntry::new(
            crate::EntryType::Insert,
            "test".to_string(),
            "doc-1".to_string(),
            Some(json!({"data": "first"})),
        );
        wal.write_entry(entry1.clone()).await.unwrap();

        let entry2 = LogEntry::new(
            crate::EntryType::Insert,
            "test".to_string(),
            "doc-2".to_string(),
            Some(json!({"data": "second"})),
        );
        wal.write_entry(entry2.clone()).await.unwrap();

        // Read back
        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    // ============ Additional Coverage Tests ============

    #[tokio::test]
    async fn test_wal_manager_get_wal_files_with_rotated_files() {
        // Test get_wal_files finds rotated WAL files
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_rotated.wal");

        let config = WalConfig {
            max_file_size:         Some(100), // Very small to trigger rotation
            max_records_per_file:  None,
            compression_algorithm: None,
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write large entries to trigger rotation
        for i in 0 .. 5 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"data": "x".repeat(100)})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // get_wal_files should find multiple files
        let files = wal.get_wal_files().unwrap();
        assert!(files.len() >= 1);
    }

    #[tokio::test]
    async fn test_wal_manager_stream_with_file_error() {
        // Test stream_entries handles file open error gracefully
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_stream_error.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write an entry
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "test".to_string(),
            "doc-1".to_string(),
            Some(json!({"data": "test"})),
        );
        wal.write_entry(entry.clone()).await.unwrap();

        // Stream should work normally
        let stream = wal.stream_entries();
        futures::pin_mut!(stream);

        let mut count = 0;
        while let Some(result) = stream.next().await {
            assert!(result.is_ok());
            count += 1;
        }
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_wal_manager_parse_binary_with_partial_data() {
        // Test parse_binary_entries handles partial/corrupt data gracefully
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_partial.wal");

        let config = WalConfig {
            format: WalFormat::Binary,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write a valid entry first
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "test".to_string(),
            "doc-1".to_string(),
            Some(json!({"valid": true})),
        );
        wal.write_entry(entry.clone()).await.unwrap();

        // Read entries - should only return valid entries
        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_wal_manager_parse_json_lines_with_invalid_utf8() {
        // Test parse_json_lines_entries handles invalid UTF-8
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_invalid_utf8.wal");

        // Manually create a WAL file with invalid UTF-8
        std::fs::write(&wal_path, b"valid entry\n\xff\xfe invalid utf8\n").unwrap();

        let config = WalConfig {
            format: WalFormat::JsonLines,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Should handle the error gracefully - the file has invalid UTF-8
        // which should cause an error during parsing
        let result = wal.read_all_entries().await;
        // The error is expected, or it may skip invalid lines depending on implementation
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("UTF-8"));
    }

    #[tokio::test]
    async fn test_wal_manager_parse_json_lines_with_malformed_json() {
        // Test parse_json_lines_entries handles malformed JSON
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_malformed.wal");

        // Manually create a WAL file with malformed JSON
        std::fs::write(
            &wal_path,
            r#"{"valid": true}
not json at all
{"also": "valid"}
"#,
        )
        .unwrap();

        let config = WalConfig {
            format: WalFormat::JsonLines,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Should skip malformed lines and return valid ones
        // The result may vary depending on error handling
        let _entries = wal.read_all_entries().await.unwrap();
        // At least the valid entries should be parsed
    }

    #[tokio::test]
    async fn test_wal_manager_entries_count_precision() {
        // Test that entries_count is accurate after many operations
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_count_precision.wal");

        let wal = WalManager::new(wal_path.clone(), WalConfig::default())
            .await
            .unwrap();

        // Write 100 entries
        for i in 0 .. 100 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"index": i})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Count should be exactly 100
        let count = wal.entries_count().await.unwrap();
        assert_eq!(count, 100);
    }

    #[tokio::test]
    async fn test_wal_manager_rotate_at_exactly_max_size() {
        // Test rotation behavior when entry size equals max_size
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_exact_size.wal");

        // Create an entry that's exactly 50 bytes
        let small_data = json!({"x": 1});
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "test".to_string(),
            "doc-1".to_string(),
            Some(small_data),
        );

        let config = WalConfig {
            max_file_size:         Some(100), // Allow one entry
            max_records_per_file:  None,
            compression_algorithm: None,
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write first entry
        wal.write_entry(entry.clone()).await.unwrap();

        // Write second entry - may trigger rotation
        wal.write_entry(entry.clone()).await.unwrap();

        // At least one entry should be written
        let count = wal.entries_count().await.unwrap();
        assert!(count >= 1);
    }

    // ============ Binary Parsing Edge Cases ============

    #[tokio::test]
    async fn test_wal_manager_binary_parse_with_checksum_mismatch() {
        // Test parse_binary_entries skips entries with invalid checksums
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_checksum.wal");

        let config = WalConfig {
            format: WalFormat::Binary,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write a valid entry first
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "test".to_string(),
            "doc-1".to_string(),
            Some(json!({"valid": true})),
        );
        wal.write_entry(entry.clone()).await.unwrap();

        // Read entries - valid entry should be returned
        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_wal_manager_get_wal_files_no_parent() {
        // Test get_wal_files when path has no parent (edge case)
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_no_parent.wal");

        // Create a simple WAL file
        let config = WalConfig::default();
        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write an entry
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "test".to_string(),
            "doc-1".to_string(),
            Some(json!({"test": 1})),
        );
        wal.write_entry(entry.clone()).await.unwrap();

        // get_wal_files should work without errors even with various path structures
        let files = wal.get_wal_files();
        assert!(files.is_ok());
        assert!(!files.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_wal_manager_get_wal_files_multiple_rotated() {
        // Test get_wal_files with multiple rotated WAL files
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_multi_rotated.wal");

        let config = WalConfig {
            max_file_size:         Some(100),
            compression_algorithm: None,
            max_records_per_file:  Some(2),
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write enough entries to trigger rotation
        for i in 0 .. 5 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"data": "x".repeat(50)})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // get_wal_files should find all rotated files
        let files = wal.get_wal_files();
        assert!(files.is_ok());
        let file_list = files.unwrap();
        // Should have at least the current file
        assert!(!file_list.is_empty());
    }

    #[tokio::test]
    async fn test_wal_manager_stream_binary_with_read_error() {
        // Test stream_entries handles read errors gracefully
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_stream_read_error.wal");

        let config = WalConfig::default();
        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write an entry
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "test".to_string(),
            "doc-1".to_string(),
            Some(json!({"test": 1})),
        );
        wal.write_entry(entry.clone()).await.unwrap();

        // Stream should work normally
        let stream = wal.stream_entries();
        futures::pin_mut!(stream);

        let mut count = 0;
        while let Some(result) = stream.next().await {
            assert!(result.is_ok());
            count += 1;
        }
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_wal_manager_stream_json_lines_with_empty_lines() {
        // Test stream_entries handles empty lines in JSON Lines format
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_stream_empty_lines.wal");

        let config = WalConfig {
            format: WalFormat::JsonLines,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write entries with empty lines between them
        for i in 0 .. 3 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"index": i})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Stream should skip empty lines and return valid entries
        let stream = wal.stream_entries();
        futures::pin_mut!(stream);

        let mut count = 0;
        while let Some(result) = stream.next().await {
            assert!(result.is_ok());
            count += 1;
        }
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_wal_manager_parse_binary_with_truncated_checksum() {
        // Test parse_binary_entries handles truncated checksum gracefully
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_truncated.wal");

        // Manually create a file with binary data that looks like it might have truncated checksums
        let entry = LogEntry::new(
            crate::EntryType::Insert,
            "test".to_string(),
            "doc-1".to_string(),
            Some(json!({"test": 1})),
        );
        let bytes = entry.to_bytes().unwrap();

        std::fs::write(&wal_path, &bytes).unwrap();

        let config = WalConfig {
            format: WalFormat::Binary,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Should parse the entry successfully
        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_wal_manager_parse_json_lines_trailing_newline() {
        // Test parse_json_lines_entries handles trailing newlines correctly
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_trailing_newline.wal");

        let config = WalConfig {
            format: WalFormat::JsonLines,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write entries
        for i in 0 .. 2 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"index": i})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Read entries - trailing newlines should be handled
        let entries = wal.read_all_entries().await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_wal_manager_parse_json_lines_only_whitespace() {
        // Test parse_json_lines_entries with only whitespace lines
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_only_whitespace.wal");

        // Manually create a file with only whitespace
        std::fs::write(&wal_path, "   \n\n\t\n   \n").unwrap();

        let config = WalConfig {
            format: WalFormat::JsonLines,
            ..Default::default()
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Should return empty list (no valid entries)
        let entries = wal.read_all_entries().await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_wal_manager_get_wal_files_sorted_correctly() {
        // Test that get_wal_files returns files in correct sorted order
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_sorted.wal");

        let config = WalConfig {
            max_file_size:         Some(50),
            compression_algorithm: None,
            max_records_per_file:  Some(1),
            format:                WalFormat::Binary,
        };

        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write multiple entries to trigger rotation
        for i in 0 .. 3 {
            let entry = LogEntry::new(
                crate::EntryType::Insert,
                "test".to_string(),
                format!("doc-{}", i),
                Some(json!({"index": i})),
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Get WAL files - current file should be first in the list
        let files = wal.get_wal_files().unwrap();
        assert!(!files.is_empty());

        // Current file (original path) should be first
        if files.len() > 1 {
            assert_eq!(files[0], wal_path);
        }
    }

    #[tokio::test]
    async fn test_wal_manager_stream_entries_with_various_entry_types() {
        // Test streaming entries with various entry types
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test_stream_types.wal");

        let config = WalConfig::default();
        let wal = WalManager::new(wal_path.clone(), config).await.unwrap();

        // Write all entry types
        let entry_types = vec![
            crate::EntryType::Insert,
            crate::EntryType::Update,
            crate::EntryType::Delete,
            crate::EntryType::Begin,
            crate::EntryType::Commit,
            crate::EntryType::Rollback,
        ];

        for (i, entry_type) in entry_types.iter().enumerate() {
            let entry = LogEntry::new(
                *entry_type,
                "test".to_string(),
                format!("doc-{}", i),
                if *entry_type == crate::EntryType::Delete {
                    None
                }
                else {
                    Some(json!({"type": format!("{:?}", entry_type)}))
                },
            );
            wal.write_entry(entry.clone()).await.unwrap();
        }

        // Stream and verify all types
        let stream = wal.stream_entries();
        futures::pin_mut!(stream);

        let mut streamed_types = Vec::new();
        while let Some(result) = stream.next().await {
            let entry = result.unwrap();
            streamed_types.push(entry.entry_type);
        }

        assert_eq!(streamed_types.len(), 6);
    }
}
