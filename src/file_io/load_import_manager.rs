//! Async load/import orchestration: background-thread canvas deserialisation
//! and image import.

use std::path::PathBuf;
use std::sync::mpsc;

use crate::canvas::Canvas;
use crate::document::Document;
use crate::undo_history::UndoHistory;

/// Result of an async load or import operation sent via channel.
///
/// Uses `Vec<Layer>` + dimensions instead of `Canvas` directly so the data
/// is [`Send`] (avoids `Canvas`'s non-Send `TextureHandle` field). The UI
/// thread reconstructs a `Canvas` from the layers when the result is polled.
pub enum LoadImportResult {
    /// Canvas loaded from a `.splattercanvas` file, plus the source path.
    Loaded(Canvas, String),
    /// Image imported as a new canvas.
    Imported(Vec<crate::canvas::Layer>, u32, u32),
    /// Canvas imported from a `.splatterarchive` file.
    ArchiveImported(Canvas),
    /// Operation failed with an error message.
    Failed(String),
}

/// Spawns background threads to read, deserialise, and import canvas files.
///
/// Owns the load/import channel pair and separate in-flight flags for
/// load vs. import operations.
pub struct LoadImportManager {
    /// Channel sender for load/import results from background thread.
    pub load_import_sender: mpsc::Sender<LoadImportResult>,
    /// Channel receiver for load/import results on the UI thread.
    pub load_import_receiver: mpsc::Receiver<LoadImportResult>,
    /// `true` while an async load thread is running.
    pub load_in_flight: bool,
    /// `true` while an async import thread is running.
    pub import_in_flight: bool,
}

impl std::fmt::Debug for LoadImportManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadImportManager")
            .field("load_in_flight", &self.load_in_flight)
            .field("import_in_flight", &self.import_in_flight)
            .finish()
    }
}

impl LoadImportManager {
    /// Create a new `LoadImportManager` with an internal channel pair.
    pub fn new() -> Self {
        let (load_import_sender, load_import_receiver) = mpsc::channel();
        Self {
            load_import_sender,
            load_import_receiver,
            load_in_flight: false,
            import_in_flight: false,
        }
    }

    /// Spawn a background thread to read and deserialise a `.splattercanvas` file.
    ///
    /// # Parameters
    ///
    /// * `path` — The file path to load.
    pub fn trigger_async_load(&mut self, path: PathBuf) {
        self.load_in_flight = true;
        let sender = self.load_import_sender.clone();
        std::thread::spawn(move || {
            let result = match crate::files::load_canvas_from_path(&path) {
                Ok(canvas) => {
                    let save_path = path.display().to_string();
                    LoadImportResult::Loaded(canvas, save_path)
                }
                Err(error) => LoadImportResult::Failed(format!("Failed to load canvas: {error}")),
            };
            let _ = sender.send(result);
        });
    }

    /// Spawn a background thread to decode and import an image file as a new canvas.
    ///
    /// # Parameters
    ///
    /// * `path` — The image file path to import.
    pub fn trigger_async_import(&mut self, path: PathBuf) {
        self.import_in_flight = true;
        let sender = self.load_import_sender.clone();
        std::thread::spawn(move || {
            let result = match crate::files::import_image_as_canvas(&path) {
                Ok(canvas) => {
                    LoadImportResult::Imported(canvas.pixels, canvas.width, canvas.height)
                }
                Err(error) => LoadImportResult::Failed(format!("Import failed: {error}")),
            };
            let _ = sender.send(result);
        });
    }

    /// Spawn a background thread to read and deserialise a `.splatterarchive` file.
    ///
    /// # Parameters
    ///
    /// * `path` — The file path to load.
    pub fn trigger_async_import_archive(&mut self, path: PathBuf) {
        self.load_in_flight = true;
        let sender = self.load_import_sender.clone();
        std::thread::spawn(move || {
            let result = match crate::files::load_canvas_from_path_xz(&path) {
                Ok(canvas) => LoadImportResult::ArchiveImported(canvas),
                Err(error) => {
                    LoadImportResult::Failed(format!("Failed to import archive: {error}"))
                }
            };
            let _ = sender.send(result);
        });
    }

    /// Poll for completed async load or import results and apply them.
    ///
    /// For a loaded canvas, replaces the document canvas and sets the save path.
    /// For an imported image, replaces the document canvas. Pushes errors
    /// to `error_list`.
    ///
    /// # Parameters
    ///
    /// * `document` — The document to modify on load/import.
    /// * `undo` — Undo history to reset on load/import.
    /// * `error_list` — Error list to push failure messages into.
    pub fn poll_load_import_results(
        &mut self,
        document: &mut Document,
        undo: &mut UndoHistory,
        error_list: &mut Vec<String>,
    ) {
        while let Ok(result) = self.load_import_receiver.try_recv() {
            match result {
                LoadImportResult::Loaded(mut canvas, save_path) => {
                    self.load_in_flight = false;
                    canvas.dirty_rect.request_full_blend();
                    document.replace_canvas(canvas, undo);
                    document.savefile_path = save_path;
                }
                LoadImportResult::Imported(layers, width, height) => {
                    self.import_in_flight = false;
                    let mut dirty_rect = crate::canvas::DirtyRectList::new();
                    dirty_rect.request_full_blend();
                    let canvas = crate::canvas::Canvas {
                        pixels: layers,
                        width,
                        height,
                        output_rgba: std::sync::Arc::new(Vec::new()),
                        rendered_layers: None,
                        dirty_rect,
                    };
                    document.replace_canvas(canvas, undo);
                }
                LoadImportResult::ArchiveImported(mut canvas) => {
                    self.load_in_flight = false;
                    canvas.dirty_rect.request_full_blend();
                    document.replace_canvas(canvas, undo);
                }
                LoadImportResult::Failed(message) => {
                    self.load_in_flight = false;
                    self.import_in_flight = false;
                    error_list.push(message);
                }
            }
        }
    }
}

impl Default for LoadImportManager {
    fn default() -> Self {
        Self::new()
    }
}
