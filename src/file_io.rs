//! Async file-dialog and save handling via mpsc channels.  Manages save-to-
//! current-path, save-as, load, and autosave workflows with result polling.

use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;

use chrono::Local;
use eframe::egui::Color32;

use crate::app::CANVAS_EXTENSION;
use crate::app::DEFAULT_CANVAS_NAME;
use crate::app::EXPORT_FORMATS;
use crate::app::FILE_FILTER_NAME;
use crate::app::IMPORT_EXTENSIONS;
use crate::document::Document;
use crate::tools::brush_parsers::BrushTip;
use crate::undo_history::UndoHistory;

// --- Autosave constants ---
const AUTOSAVE_DIRECTORY: &str = "autosaves";
const AUTOSAVE_DATE_FORMAT: &str = "%Y-%m-%d_%H-%M-%S";

// --- File-dialog types ---

/// A file-dialog action queued for execution on a background thread.
/// The result is received via channel at the start of a future frame.
#[derive(Clone, Copy)]
pub enum PendingFileAction {
    /// Open a native "load" dialog and deserialize a `.splattercanvas` file.
    Load,
    /// Open a native "save" dialog and serialize the current canvas.
    Save,
    /// Open a native "open" dialog for importing an image as a new canvas.
    Import,
    /// Open a native "save" dialog for exporting to one of the supported image
    /// formats. The `usize` payload indexes into `EXPORT_FORMATS`.
    Export(usize),
    /// Open a native "open" dialog to load an image as a stamp.
    LoadStamp,
    /// Open a native "open" dialog to load a brush file (.abr, .gbr, .brush).
    LoadBrush,
}

/// Message sent back from the file-dialog thread to the UI thread.
pub enum DialogResult {
    Picked(PathBuf),
    /// Decoded stamp image pixels + dimensions + suggested name (file stem).
    StampPixels(Vec<Color32>, u32, u32, String),
    /// Parsed brush tips from an ABR/GBR file.
    BrushTips(Vec<BrushTip>),
    /// An error occurred during a file operation.
    Error(String),
}

/// Distinguishes an autosave from a manual save in the async save pipeline.
pub enum SaveKind {
    /// Periodic autosave to `{data_dir}/autosaves/`.
    Autosave,
    /// Explicit user-initiated save to a chosen path.
    ManualSave(PathBuf),
}

/// Result sent back via channel when an async save completes.
#[derive(Debug)]
pub enum SaveResult {
    /// Autosave completed successfully (resulting path is not surfaced).
    Autosave,
    /// Manual save completed to the given path.
    ManualSave(PathBuf),
    /// Save failed with an error message.
    Failed(String),
}

/// Manages async file dialogs and save operations via background threads.
///
/// Holds channel pairs for receiving dialog results and save outcomes,
/// plus the app's local data directory path for autosaves.
pub struct FileIO {
    /// File action queued for the next background thread iteration.
    pub pending_file_action: Option<PendingFileAction>,
    /// Channel sender for dispatching dialog requests to the background thread.
    pub dialog_sender: mpsc::Sender<DialogResult>,
    /// Channel receiver for receiving dialog results on the UI thread.
    pub dialog_receiver: mpsc::Receiver<DialogResult>,
    /// Channel sender for dispatching save requests to the background thread.
    pub save_result_sender: mpsc::Sender<SaveResult>,
    /// Channel receiver for receiving save results on the UI thread.
    pub save_result_receiver: mpsc::Receiver<SaveResult>,
    /// Base path for autosave directory (`{data_dir}/autosaves/`).
    pub app_local_data_directory: PathBuf,
    /// Result of a stamp-image load, consumed by the app frame after polling.
    /// Contains pixels, width, height, and suggested name.
    pub loaded_stamp_data: Option<(Vec<Color32>, u32, u32, String)>,
    /// Result of a brush-file load, consumed by the app frame after polling.
    /// Contains parsed brush tips.
    pub loaded_brush_data: Option<Vec<BrushTip>>,
}

impl FileIO {
    /// Create a new `FileIO` with channel pairs and an app data directory path.
    ///
    /// `dialog_sender`/`dialog_receiver` are used for file dialog results.
    /// `save_result_sender`/`save_result_receiver` are used for async save outcomes.
    ///
    /// # Parameters
    ///
    /// * `dialog_sender` — Channel sender for file-dialog results.
    /// * `dialog_receiver` — Channel receiver for file-dialog results.
    /// * `save_result_sender` — Channel sender for async save results.
    /// * `save_result_receiver` — Channel receiver for async save results.
    /// * `app_local_data_directory` — Base path for autosave directory.
    pub fn new(
        dialog_sender: mpsc::Sender<DialogResult>,
        dialog_receiver: mpsc::Receiver<DialogResult>,
        save_result_sender: mpsc::Sender<SaveResult>,
        save_result_receiver: mpsc::Receiver<SaveResult>,
        app_local_data_directory: PathBuf,
    ) -> Self {
        Self {
            pending_file_action: None,
            dialog_sender,
            dialog_receiver,
            save_result_sender,
            save_result_receiver,
            app_local_data_directory,
            loaded_stamp_data: None,
            loaded_brush_data: None,
        }
    }

    /// Queue a file dialog action and spawn it on a background thread.
    ///
    /// The dialog is dispatched to the main thread via `rfd` to avoid macOS
    /// winit re-entrancy panics. Supports Save, Load, Import, and Export
    /// actions with appropriate file filters and default names.
    ///
    /// # Parameters
    ///
    /// * `action` — The file-dialog action to perform.
    pub fn queue_file_action(&mut self, action: PendingFileAction) {
        let sender = self.dialog_sender.clone();

        match action {
            PendingFileAction::Save => {
                self.pending_file_action = Some(PendingFileAction::Save);
                std::thread::spawn(move || {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(
                            FILE_FILTER_NAME,
                            &[CANVAS_EXTENSION.trim_start_matches('.')],
                        )
                        .set_file_name(DEFAULT_CANVAS_NAME)
                        .save_file()
                    {
                        let _ = sender.send(DialogResult::Picked(path));
                    }
                });
            }
            PendingFileAction::Load => {
                self.pending_file_action = Some(PendingFileAction::Load);
                std::thread::spawn(move || {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(
                            FILE_FILTER_NAME,
                            &[CANVAS_EXTENSION.trim_start_matches('.')],
                        )
                        .pick_file()
                    {
                        let _ = sender.send(DialogResult::Picked(path));
                    }
                });
            }
            PendingFileAction::Import => {
                self.pending_file_action = Some(PendingFileAction::Import);
                std::thread::spawn(move || {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", IMPORT_EXTENSIONS)
                        .pick_file()
                    {
                        let _ = sender.send(DialogResult::Picked(path));
                    }
                });
            }
            PendingFileAction::Export(index) => {
                self.pending_file_action = Some(PendingFileAction::Export(index));
                let information = &EXPORT_FORMATS[index].1;
                let extensions: Vec<&str> = information.extensions.to_vec();
                let default_name = format!("export.{}", information.extensions[0]);
                std::thread::spawn(move || {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(EXPORT_FORMATS[index].0, &extensions)
                        .set_file_name(&default_name)
                        .save_file()
                    {
                        let _ = sender.send(DialogResult::Picked(path));
                    }
                });
            }
            PendingFileAction::LoadBrush => {
                self.pending_file_action = Some(PendingFileAction::LoadBrush);
                std::thread::spawn(move || {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Brush Files", &["abr", "gbr"])
                        .pick_file()
                    {
                        // Parse the brush file in the background thread
                        match crate::tools::brush_parsers::parse_brush_file(&path) {
                            Ok(tips) => {
                                let _ = sender.send(DialogResult::BrushTips(tips));
                            }
                            Err(error) => {
                                let _ = sender.send(DialogResult::Error(format!(
                                    "Failed to load brush: {error}"
                                )));
                            }
                        }
                    }
                });
            }
            PendingFileAction::LoadStamp => {
                self.pending_file_action = Some(PendingFileAction::LoadStamp);
                std::thread::spawn(move || {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", IMPORT_EXTENSIONS)
                        .pick_file()
                    {
                        // Decode the image in the background thread to avoid
                        // blocking the UI frame.
                        match image::open(&path) {
                            Ok(dynamic_image) => {
                                let rgba = dynamic_image.to_rgba8();
                                let (w, h) = rgba.dimensions();
                                let pixel_count = (w as usize) * (h as usize);
                                let mut pixels = Vec::with_capacity(pixel_count);
                                for pixel in rgba.pixels() {
                                    let straight = Color32::from_rgba_unmultiplied(
                                        pixel[0], pixel[1], pixel[2], pixel[3],
                                    );
                                    pixels.push(straight);
                                }
                                let name = path
                                    .file_stem()
                                    .map(|s| s.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| "stamp".to_string());
                                let _ = sender.send(DialogResult::StampPixels(pixels, w, h, name));
                            }
                            Err(error) => {
                                let _ = sender.send(DialogResult::Error(format!(
                                    "Failed to load stamp: {error}"
                                )));
                            }
                        }
                    }
                });
            }
        }
    }

    /// Poll for completed file dialog results and process them.
    ///
    /// Called once per frame before egui layout. Handles load, import,
    /// save, and export actions by reading/writing files and updating
    /// document state or the error list accordingly.
    ///
    /// # Parameters
    ///
    /// * `document` — The document to modify on load/import.
    /// * `undo` — Undo history to reset on load/import.
    /// * `error_list` — Error list to push failure messages into.
    pub fn poll_dialog_results(
        &mut self,
        document: &mut Document,
        undo: &mut UndoHistory,
        error_list: &mut Vec<String>,
    ) {
        while let Ok(result) = self.dialog_receiver.try_recv() {
            match result {
                DialogResult::StampPixels(pixels, w, h, name) => {
                    self.loaded_stamp_data = Some((pixels, w, h, name));
                    self.pending_file_action = None;
                }
                DialogResult::BrushTips(tips) => {
                    self.loaded_brush_data = Some(tips);
                    self.pending_file_action = None;
                }
                DialogResult::Error(msg) => {
                    error_list.push(msg);
                    self.pending_file_action = None;
                }
                DialogResult::Picked(path) => {
                    let Some(pending) = self.pending_file_action.take() else {
                        continue;
                    };
                    match pending {
                        PendingFileAction::Save => {
                            let path_string = path.display().to_string();
                            let save_path = if path_string.ends_with(CANVAS_EXTENSION) {
                                path
                            } else {
                                PathBuf::from(format!("{path_string}{CANVAS_EXTENSION}"))
                            };
                            self.trigger_async_save(document, SaveKind::ManualSave(save_path));
                        }
                        PendingFileAction::Load => match crate::files::load_data_from_file(&path) {
                            Ok(data) => match crate::files::load_app_from_data(&data) {
                                Ok(canvas) => {
                                    let save_path = path.display().to_string();
                                    document.replace_canvas(canvas, undo);
                                    document.savefile_path = save_path;
                                }
                                Err(error) => {
                                    error_list.push(format!("Failed to load canvas: {error}"))
                                }
                            },
                            Err(error) => error_list.push(format!("Failed to read file: {error}")),
                        },
                        PendingFileAction::Import => {
                            match crate::files::import_image_as_canvas(&path) {
                                Ok(canvas) => document.replace_canvas(canvas, undo),
                                Err(error) => error_list.push(format!("Import failed: {error}")),
                            }
                        }
                        PendingFileAction::Export(index) => {
                            if document.canvas.output_rgba.is_empty() {
                                continue;
                            }
                            let information = &EXPORT_FORMATS[index].1;
                            let default_extension = information.extensions[0];
                            let path_string = path.display().to_string();
                            let path_string = if information
                                .extensions
                                .iter()
                                .any(|ext| path_string.ends_with(ext))
                            {
                                path_string
                            } else {
                                format!("{path_string}.{default_extension}")
                            };
                            if let Err(error) = crate::files::export_as_image(
                                &document.canvas.output_rgba,
                                document.canvas.width,
                                document.canvas.height,
                                Path::new(&path_string),
                                information.fmt,
                            ) {
                                error_list.push(format!("Export failed: {error}"));
                            }
                        }
                        PendingFileAction::LoadStamp => {
                            // Handled exclusively by the StampPixels variant above.
                        }
                        PendingFileAction::LoadBrush => {
                            // Handled exclusively by the BrushTips variant above.
                        }
                    }
                }
            }
        }
    }

    /// Spawn a background thread to serialize and write the canvas to disk.
    ///
    /// The thread clones the current canvas to avoid blocking the UI.
    /// For autosaves, the file name is a timestamp under `AUTOSAVE_DIRECTORY`.
    /// For manual saves, the provided path is used. Results are sent back
    /// via `save_result_sender`.
    ///
    /// # Parameters
    ///
    /// * `document` — The document whose canvas will be saved.
    /// * `kind` — Whether this is an autosave or a manual save to a specific path.
    pub fn trigger_async_save(&self, document: &Document, kind: SaveKind) {
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
            let result = match crate::files::save_canvas_to_bytes(&canvas) {
                Ok(data) => match crate::files::save_bytes_to_file(&data, &path) {
                    Ok(()) => match kind {
                        SaveKind::Autosave => SaveResult::Autosave,
                        SaveKind::ManualSave(_) => SaveResult::ManualSave(path),
                    },
                    Err(error) => SaveResult::Failed(format!("Write failed: {error}")),
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
    pub fn save_to_current_path(&self, document: &Document) {
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
    /// after manual save, and pushes errors to the error list.
    ///
    /// # Parameters
    ///
    /// * `document` — The document to update save-path / dirty-flag on.
    /// * `error_list` — Error list to push failure messages into.
    pub fn poll_save_results(&self, document: &mut Document, error_list: &mut Vec<String>) {
        while let Ok(result) = self.save_result_receiver.try_recv() {
            match result {
                SaveResult::Autosave => {
                    document.dirty_since_last_autosave = false;
                }
                SaveResult::ManualSave(path) => {
                    document.savefile_path = path.display().to_string();
                    document.canvas_mut().render_next_frame = true;
                }
                SaveResult::Failed(message) => {
                    error_list.push(format!("Save failed: {message}"));
                }
            }
        }
    }
}
