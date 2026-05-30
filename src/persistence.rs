//! Config persistence: save/load tool configuration, recent files,
//! and periodic config write on the autosave cadence.

use std::path::PathBuf;

use crate::app::MyApp;
use crate::app::PersistedConfig;
use serde_json;

impl MyApp {
    /// Add a file path to the recent-files list (dedup, max 10, most recent first).
    pub(crate) fn push_recent_file(&mut self, path: PathBuf) {
        if path.as_os_str().is_empty() {
            return;
        }
        self.ui.recent_files.retain(|p| p != &path);
        self.ui.recent_files.insert(0, path);
        self.ui.recent_files.truncate(10);
    }

    /// Path to the user-config JSON file (tool settings, preferences).
    pub(crate) fn config_path(&self) -> PathBuf {
        self.file_io.app_local_data_directory.join("config.json")
    }

    /// Persist current tool configuration and recent files to disk.
    pub(crate) fn save_config(&self) {
        let persisted = PersistedConfig {
            tool_configuration: self.tool_configuration.clone(),
            recent_files: self.ui.recent_files.clone(),
        };
        let path = self.config_path();
        if let Ok(json) = serde_json::to_string(&persisted) {
            let _ = std::fs::write(&path, json);
        }
    }
}
