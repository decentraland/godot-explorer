//! Configuration for the scene logging system.

use godot::classes::Os;
use godot::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for the scene logging system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneLoggingConfig {
    /// Whether scene logging is enabled at runtime.
    pub enabled: bool,
    /// Directory where log files are stored.
    pub log_directory: PathBuf,
    /// Maximum size of a single log file in megabytes before rotation.
    pub max_file_size_mb: u64,
    /// Maximum total size of all log files in megabytes.
    pub max_total_size_mb: u64,
    /// Port for the HTTP server serving the web frontend.
    pub server_port: u16,
    /// Whether the HTTP server is enabled.
    pub server_enabled: bool,
    /// Whether CRDT message logging is enabled.
    pub crdt_logging_enabled: bool,
    /// Whether op call logging is enabled.
    pub op_logging_enabled: bool,
    /// Maximum size of response bodies to log (in bytes). Larger bodies are truncated.
    pub truncate_body_bytes: usize,
}

impl Default for SceneLoggingConfig {
    fn default() -> Self {
        // Use an accessible directory for log files
        // On Android: Documents folder (/storage/emulated/0/Documents/) - accessible via file managers
        // On Desktop: User data directory
        let log_directory = {
            let os = Os::singleton();
            let os_name = os.get_name().to_string();

            if os_name == "Android" {
                // Use Documents folder on Android for easy access and sharing
                let docs_dir = os
                    .get_system_dir(godot::classes::os::SystemDir::DOCUMENTS)
                    .to_string();
                PathBuf::from(docs_dir).join("DecentralandSceneLogs")
            } else {
                // Use user data directory on other platforms
                let user_dir = os.get_user_data_dir().to_string();
                PathBuf::from(user_dir).join("scene_logs")
            }
        };

        Self {
            enabled: true,
            log_directory,
            max_file_size_mb: 100,
            max_total_size_mb: 1024, // 1 GB
            server_port: 9876,
            server_enabled: true,
            crdt_logging_enabled: true,
            op_logging_enabled: true,
            truncate_body_bytes: 10 * 1024, // 10 KB
        }
    }
}

impl SceneLoggingConfig {
    /// Creates a new configuration with the specified log directory.
    pub fn with_log_directory(mut self, path: PathBuf) -> Self {
        self.log_directory = path;
        self
    }

    /// Creates a new configuration with the specified server port.
    pub fn with_server_port(mut self, port: u16) -> Self {
        self.server_port = port;
        self
    }

    /// Disables the HTTP server.
    pub fn without_server(mut self) -> Self {
        self.server_enabled = false;
        self
    }

    /// Disables CRDT logging.
    pub fn without_crdt_logging(mut self) -> Self {
        self.crdt_logging_enabled = false;
        self
    }

    /// Disables op call logging.
    pub fn without_op_logging(mut self) -> Self {
        self.op_logging_enabled = false;
        self
    }
}
