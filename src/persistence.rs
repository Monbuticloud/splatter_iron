//! Config persistence: save/load tool configuration, recent files,
//! and periodic config write on the autosave cadence.

use std::path::PathBuf;

use crate::app::MyApp;

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
}
