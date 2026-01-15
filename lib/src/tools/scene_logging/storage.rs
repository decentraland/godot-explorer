//! JSONL storage manager for scene logs.

use super::{config::SceneLoggingConfig, SceneLogEntry};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::PathBuf,
};

/// Manages JSONL file storage for scene logs.
pub struct StorageManager {
    config: SceneLoggingConfig,
    current_file: Option<BufWriter<File>>,
    current_file_path: PathBuf,
    current_file_size: u64,
    session_id: String,
}

impl StorageManager {
    /// Creates a new storage manager.
    pub fn new(config: SceneLoggingConfig, session_id: String) -> io::Result<Self> {
        // Create log directory if it doesn't exist
        fs::create_dir_all(&config.log_directory)?;

        let file_path = config.log_directory.join(format!("{}.jsonl", session_id));

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        Ok(Self {
            config,
            current_file: Some(BufWriter::with_capacity(64 * 1024, file)), // 64KB buffer
            current_file_path: file_path,
            current_file_size: 0,
            session_id,
        })
    }

    /// Writes a log entry to the current file.
    pub fn write_entry(&mut self, entry: &SceneLogEntry) -> io::Result<()> {
        if let Some(ref mut writer) = self.current_file {
            let json = serde_json::to_string(entry)?;
            let bytes = json.as_bytes();

            writer.write_all(bytes)?;
            writer.write_all(b"\n")?;

            self.current_file_size += bytes.len() as u64 + 1;

            // Check if we need to rotate
            if self.current_file_size >= self.config.max_file_size_mb * 1024 * 1024 {
                self.rotate()?;
            }
        }
        Ok(())
    }

    /// Flushes the current file buffer.
    pub fn flush(&mut self) -> io::Result<()> {
        if let Some(ref mut writer) = self.current_file {
            writer.flush()?;
        }
        Ok(())
    }

    /// Rotates the current log file.
    fn rotate(&mut self) -> io::Result<()> {
        // Flush and close current file
        if let Some(ref mut writer) = self.current_file {
            writer.flush()?;
        }
        self.current_file = None;

        // Rename current file with timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let rotated_path = self
            .config
            .log_directory
            .join(format!("{}_{}.jsonl", self.session_id, timestamp));
        fs::rename(&self.current_file_path, &rotated_path)?;

        // Create new file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.current_file_path)?;

        self.current_file = Some(BufWriter::with_capacity(64 * 1024, file));
        self.current_file_size = 0;

        // Cleanup old files if total size exceeds limit
        self.cleanup_old_files()?;

        Ok(())
    }

    /// Removes old log files if total size exceeds the configured limit.
    fn cleanup_old_files(&self) -> io::Result<()> {
        let max_bytes = self.config.max_total_size_mb * 1024 * 1024;
        let mut files: Vec<_> = fs::read_dir(&self.config.log_directory)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "jsonl")
                    .unwrap_or(false)
            })
            .collect();

        // Sort by modification time (oldest first)
        files.sort_by(|a, b| {
            let a_time = a.metadata().and_then(|m| m.modified()).ok();
            let b_time = b.metadata().and_then(|m| m.modified()).ok();
            a_time.cmp(&b_time)
        });

        // Calculate total size
        let mut total_size: u64 = files
            .iter()
            .filter_map(|f| f.metadata().ok())
            .map(|m| m.len())
            .sum();

        // Remove oldest files until under limit
        for entry in files {
            if total_size <= max_bytes {
                break;
            }
            if let Ok(metadata) = entry.metadata() {
                let file_size = metadata.len();
                // Don't delete the current session file
                if entry.path() != self.current_file_path {
                    fs::remove_file(entry.path())?;
                    total_size -= file_size;
                    tracing::info!("Removed old log file: {:?}", entry.path());
                }
            }
        }

        Ok(())
    }

    /// Lists all log files in the log directory.
    pub fn list_log_files(&self) -> io::Result<Vec<PathBuf>> {
        let mut files: Vec<_> = fs::read_dir(&self.config.log_directory)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|ext| ext == "jsonl").unwrap_or(false))
            .collect();

        // Sort by modification time (newest first)
        files.sort_by(|a, b| {
            let a_time = fs::metadata(a).and_then(|m| m.modified()).ok();
            let b_time = fs::metadata(b).and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        Ok(files)
    }

    /// Gets the path to the current log file.
    pub fn current_file_path(&self) -> &PathBuf {
        &self.current_file_path
    }

    /// Gets the session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

impl Drop for StorageManager {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
