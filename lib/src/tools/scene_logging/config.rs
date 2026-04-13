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
        }
    }
}

