//! Config persistence: save/load tool configuration, recent files,
//! and periodic config write on the autosave cadence.

use std::path::PathBuf;
use std::time::Duration;

use serde_json;

use crate::app::AUTOSAVE_INTERVAL_MINUTES;
use crate::app::MyApp;
use crate::app::PersistedConfig;

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
        self.save_manager
            .app_local_data_directory
            .join("config.json")
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

    /// Persist tool configuration to disk (runs on the same cadence as autosave).
    pub(crate) fn handle_config_save(&mut self) {
        if self
            .ui
            .time_elapsed
            .saturating_sub(self.ui.last_autosave_time)
            >= Duration::from_mins(AUTOSAVE_INTERVAL_MINUTES)
        {
            self.save_config();
        }
    }
}
