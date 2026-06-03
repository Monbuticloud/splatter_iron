//! Async save orchestration: background-thread serialisation for manual saves
//! and periodic autosaves.

use std::path::PathBuf;
use std::sync::mpsc;

use chrono::Local;

use crate::document::Document;

// --- Autosave constants ---
const AUTOSAVE_DIRECTORY: &str = "autosaves";
const AUTOSAVE_DATE_FORMAT: &str = "%Y-%m-%d_%H-%M-%S";

/// Distinguishes an autosave from a manual save in the async save pipeline.
pub enum SaveKind {
    /// Periodic autosave to `{data_dir}/autosaves/`.
    Autosave,
    /// Explicit user-initiated save to a chosen path.
    ManualSave(PathBuf),
}

/// Result of an async save operation sent via channel.
#[derive(Debug)]
pub enum SaveResult {
    /// Autosave completed successfully.
    Autosave,
    /// Manual save completed to the given path.
    ManualSave(PathBuf),
    /// Save failed with an error message.
    Failed(String),
}

/// Spawns background threads to serialise and write canvas files.
///
/// Owns the save-result channel pair and the in-flight flag. The
/// `app_local_data_directory` is used to construct autosave paths.
pub struct SaveManager {
    /// Channel sender for save results from background thread to UI thread.
    pub save_result_sender: mpsc::Sender<SaveResult>,
    /// Channel receiver for save results on the UI thread.
    pub save_result_receiver: mpsc::Receiver<SaveResult>,
    /// Base path for autosave directory (`{data_dir}/autosaves/`).
    pub app_local_data_directory: PathBuf,
    /// `true` when the most recently triggered async save is an autosave.
    /// Used by the UI to display "Autosaving…" vs "Saving…" in the status bar.
    pub autosave_in_flight: bool,
}

impl std::fmt::Debug for SaveManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaveManager")
            .field("autosave_in_flight", &self.autosave_in_flight)
            .finish()
    }
}

impl SaveManager {
    /// Create a new `SaveManager`.
    ///
    /// # Parameters
    ///
    /// * `save_result_sender` — Channel sender for async save results.
    /// * `save_result_receiver` — Channel receiver for async save results.
    /// * `app_local_data_directory` — Base path under which the `autosaves/`
    ///   subdirectory exists.
    pub fn new(
        save_result_sender: mpsc::Sender<SaveResult>,
        save_result_receiver: mpsc::Receiver<SaveResult>,
        app_local_data_directory: PathBuf,
    ) -> Self {
        Self {
            save_result_sender,
            save_result_receiver,
            app_local_data_directory,
            autosave_in_flight: false,
        }
    }

    /// Return the path to the autosave directory (`{data_dir}/autosaves/`).
    pub fn autosave_directory(&self) -> PathBuf {
        self.app_local_data_directory.join(AUTOSAVE_DIRECTORY)
    }

    /// Spawn a background thread to serialise and write the canvas to disk.
    ///
    /// The thread clones the current canvas to avoid blocking the UI.
    /// For autosaves, the file name is a timestamp under the autosave
    /// directory. For manual saves, the provided path is used. Results
    /// are sent back via `save_result_sender`.
    ///
    /// # Parameters
    ///
    /// * `document` — The document whose canvas will be saved.
    /// * `kind` — Whether this is an autosave or a manual save to a specific path.
    pub fn trigger_async_save(&mut self, document: &mut Document, kind: SaveKind) {
        use crate::document::SaveState;
        self.autosave_in_flight = matches!(kind, SaveKind::Autosave);
        document.save_state = SaveState::InFlight;
        let canvas = document.canvas.clone();
        let path = match &kind {
            SaveKind::Autosave => {
                self.app_local_data_directory
                    .join(AUTOSAVE_DIRECTORY)
                    .join(format!(
                        "{}.splattercanvas",
                        Local::now().format(AUTOSAVE_DATE_FORMAT)
                    ))
            }
            SaveKind::ManualSave(save_path) => save_path.clone(),
        };
        let sender = self.save_result_sender.clone();
        std::thread::spawn(move || {
            let result = match crate::files::save_canvas_to_path(&canvas, &path) {
                Ok(()) => match kind {
                    SaveKind::Autosave => SaveResult::Autosave,
                    SaveKind::ManualSave(_) => SaveResult::ManualSave(path),
                },
                Err(error) => SaveResult::Failed(format!("Serialisation failed: {error}")),
            };
            let _ = sender.send(result);
        });
    }

    /// Save to the current `savefile_path` asynchronously.
    ///
    /// No-op if `savefile_path` is empty.
    ///
    /// # Parameters
    ///
    /// * `document` — The document whose canvas will be saved.
    pub fn save_to_current_path(&mut self, document: &mut Document) {
        if !document.savefile_path.is_empty() {
            self.trigger_async_save(
                document,
                SaveKind::ManualSave(PathBuf::from(&document.savefile_path)),
            );
        }
    }

    /// Poll for completed async save results and update state accordingly.
    ///
    /// Marks the document as clean after autosave, sets the save path
    /// after manual save, pushes errors to the error list, and resets
    /// [`SaveState`](crate::document::SaveState) to `Idle`.
    ///
    /// # Parameters
    ///
    /// * `document` — The document to update save-path / dirty-flag on.
    /// * `error_list` — Error list to push failure messages into.
    pub fn poll_save_results(&mut self, document: &mut Document, error_list: &mut Vec<String>) {
        while let Ok(result) = self.save_result_receiver.try_recv() {
            match result {
                SaveResult::Autosave => {
                    document.dirty_since_last_autosave = false;
                }
                SaveResult::ManualSave(path) => {
                    document.savefile_path = path.display().to_string();
                    document.dirty_since_last_autosave = false;
                    document.canvas_mut().dirty_rect.request_full_blend();
                }
                SaveResult::Failed(message) => {
                    error_list.push(format!("Save failed: {message}"));
                }
            }
            document.save_state = crate::document::SaveState::Idle;
            self.autosave_in_flight = false;
        }
    }
}
