use std::path::{ Path, PathBuf };
use std::sync::mpsc;

use chrono::Local;

use crate::app::{
    CANVAS_EXT, EXPORT_FORMATS, FILE_FILTER_NAME, DEFAULT_CANVAS_NAME,
    IMPORT_EXTENSIONS,
};
use crate::document::Document;
use crate::undo_history::UndoHistory;

// --- Autosave constants ---
const AUTOSAVE_DIR: &str = "autosaves";
const AUTOSAVE_DATE_FMT: &str = "%Y-%m-%d_%H-%M-%S";

// --- File-dialog types ---

/// A file-dialog action queued for execution on a background thread.
/// The result is received via channel at the start of a future frame.
#[derive(Clone, Copy)]
pub enum PendingFileAction {
    Load,
    Save,
    Import,
    Export(usize),
}

/// Message sent back from the file-dialog thread to the UI thread.
pub enum DialogResult {
    Picked(PathBuf),
}

/// Distinguishes an autosave from a manual save in the async save pipeline.
pub enum SaveKind {
    Autosave,
    ManualSave(PathBuf),
}

/// Result sent back via channel when an async save completes.
pub enum SaveResult {
    Autosave,
    ManualSave(PathBuf),
    Failed(String),
}

/// Manages async file dialogs and save operations via background threads.
///
/// Holds channel pairs for receiving dialog results and save outcomes,
/// plus the app's local data directory path for autosaves.
pub struct FileIO {
    pub pending_file_action: Option<PendingFileAction>,
    pub dialog_sender: mpsc::Sender<DialogResult>,
    pub dialog_receiver: mpsc::Receiver<DialogResult>,
    pub save_result_sender: mpsc::Sender<SaveResult>,
    pub save_result_receiver: mpsc::Receiver<SaveResult>,
    pub app_local_data_directory: PathBuf,
}

impl FileIO {
    /// Create a new `FileIO` with channel pairs and an app data directory path.
    ///
    /// `dialog_sender`/`dialog_receiver` are used for file dialog results.
    /// `save_result_sender`/`save_result_receiver` are used for async save outcomes.
    pub fn new(
        dialog_sender: mpsc::Sender<DialogResult>,
        dialog_receiver: mpsc::Receiver<DialogResult>,
        save_result_sender: mpsc::Sender<SaveResult>,
        save_result_receiver: mpsc::Receiver<SaveResult>,
        app_local_data_directory: PathBuf,
    ) -> Self
    {
        Self {
            pending_file_action: None,
            dialog_sender,
            dialog_receiver,
            save_result_sender,
            save_result_receiver,
            app_local_data_directory,
        }
    }

    /// Queue a file dialog action and spawn it on a background thread.
    ///
    /// The dialog is dispatched to the main thread via `rfd` to avoid macOS
    /// winit re-entrancy panics. Supports Save, Load, Import, and Export
    /// actions with appropriate file filters and default names.
    pub fn queue_file_action(&mut self, action: PendingFileAction) {
        let tx = self.dialog_sender.clone();

        match action {
            PendingFileAction::Save => {
                self.pending_file_action = Some(PendingFileAction::Save);
                std::thread::spawn(move || {
                    if
                        let Some(path) = rfd::FileDialog
                            ::new()
                            .add_filter(FILE_FILTER_NAME, &[CANVAS_EXT.trim_start_matches('.')])
                            .set_file_name(DEFAULT_CANVAS_NAME)
                            .save_file()
                    {
                        let _ = tx.send(DialogResult::Picked(path));
                    }
                });
            }
            PendingFileAction::Load => {
                self.pending_file_action = Some(PendingFileAction::Load);
                std::thread::spawn(move || {
                    if
                        let Some(path) = rfd::FileDialog
                            ::new()
                            .add_filter(FILE_FILTER_NAME, &[CANVAS_EXT.trim_start_matches('.')])
                            .pick_file()
                    {
                        let _ = tx.send(DialogResult::Picked(path));
                    }
                });
            }
            PendingFileAction::Import => {
                self.pending_file_action = Some(PendingFileAction::Import);
                std::thread::spawn(move || {
                    if
                        let Some(path) = rfd::FileDialog
                            ::new()
                            .add_filter("Images", IMPORT_EXTENSIONS)
                            .pick_file()
                    {
                        let _ = tx.send(DialogResult::Picked(path));
                    }
                });
            }
            PendingFileAction::Export(idx) => {
                self.pending_file_action = Some(PendingFileAction::Export(idx));
                let info = &EXPORT_FORMATS[idx].1;
                let exts: Vec<&str> = info.extensions.to_vec();
                let default_name = format!("export.{}", info.extensions[0]);
                std::thread::spawn(move || {
                    if
                        let Some(path) = rfd::FileDialog
                            ::new()
                            .add_filter(EXPORT_FORMATS[idx].0, &exts)
                            .set_file_name(&default_name)
                            .save_file()
                    {
                        let _ = tx.send(DialogResult::Picked(path));
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
    pub fn poll_dialog_results(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoHistory,
        error_list: &mut Vec<String>,
    ) {
        while let Ok(result) = self.dialog_receiver.try_recv() {
            match result {
                DialogResult::Picked(path) => {
                    let Some(pending) = self.pending_file_action.take() else {
                        continue;
                    };
                    match pending {
                        PendingFileAction::Save => {
                            let path_str = path.display().to_string();
                            let savepath = if path_str.ends_with(CANVAS_EXT) {
                                path
                            } else {
                                PathBuf::from(format!("{path_str}{CANVAS_EXT}"))
                            };
                            self.trigger_async_save(doc, SaveKind::ManualSave(savepath));
                        }
                        PendingFileAction::Load => {
                            match crate::files::load_data_from_file(&path) {
                                Ok(data) => {
                                    match crate::files::load_app_from_data(&data) {
                                        Ok(canvas) => {
                                            let save_path = path.display().to_string();
                                            doc.replace_canvas(canvas, undo);
                                            doc.savefile_path = save_path;
                                        }
                                        Err(e) => error_list.push(
                                            format!("Failed to load canvas: {e}")
                                        ),
                                    }
                                }
                                Err(e) => error_list.push(
                                    format!("Failed to read file: {e}")
                                ),
                            }
                        }
                        PendingFileAction::Import => {
                            match crate::files::import_image_as_canvas(&path) {
                                Ok(canvas) => doc.replace_canvas(canvas, undo),
                                Err(e) => error_list.push(
                                    format!("Import failed: {e}")
                                ),
                            }
                        }
                        PendingFileAction::Export(idx) => {
                            if doc.canvas.output_rgba.is_empty() {
                                continue;
                            }
                            let info = &EXPORT_FORMATS[idx].1;
                            let default_ext = info.extensions[0];
                            let path_str = path.display().to_string();
                            let path_str = if
                                info.extensions.iter().any(|ext| path_str.ends_with(ext))
                            {
                                path_str
                            } else {
                                format!("{path_str}.{default_ext}")
                            };
                            if
                                let Err(e) = crate::files::export_as_image(
                                    &doc.canvas.output_rgba,
                                    doc.canvas.width,
                                    doc.canvas.height,
                                    Path::new(&path_str),
                                    info.fmt
                                )
                            {
                                error_list.push(
                                    format!("Export failed: {e}")
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Spawn a background thread to serialize and write the canvas to disk.
    ///
    /// The thread clones the current canvas to avoid blocking the UI.
    /// For autosaves, the file name is a timestamp under `AUTOSAVE_DIR`.
    /// For manual saves, the provided path is used. Results are sent back
    /// via `save_result_sender`.
    pub fn trigger_async_save(&self, doc: &Document, kind: SaveKind) {
        let canvas = doc.canvas.clone();
        let path = match &kind {
            SaveKind::Autosave =>
                self.app_local_data_directory
                    .join(AUTOSAVE_DIR)
                    .join(format!("{}.splattercanvas", Local::now().format(AUTOSAVE_DATE_FMT))),
            SaveKind::ManualSave(p) => p.clone(),
        };
        let tx = self.save_result_sender.clone();
        std::thread::spawn(move || {
            let result = match crate::files::save_canvas_to_bytes(&canvas) {
                Ok(data) =>
                    match crate::files::save_bytes_to_file(&data, &path) {
                        Ok(()) =>
                            match kind {
                                SaveKind::Autosave => SaveResult::Autosave,
                                SaveKind::ManualSave(_) => SaveResult::ManualSave(path),
                            }
                        Err(e) => SaveResult::Failed(format!("Write failed: {e}")),
                    }
                Err(e) => SaveResult::Failed(format!("Serialisation failed: {e}")),
            };
            let _ = tx.send(result);
        });
    }

    /// Save to the current `savefile_path` asynchronously.
    ///
    /// No-op if `savefile_path` is empty.
    pub fn save_to_current_path(&self, doc: &Document) {
        if !doc.savefile_path.is_empty() {
            self.trigger_async_save(doc, SaveKind::ManualSave(PathBuf::from(&doc.savefile_path)));
        }
    }

    /// Poll for completed async save results and update state accordingly.
    ///
    /// Marks the document as clean after autosave, sets the save path
    /// after manual save, and pushes errors to the error list.
    pub fn poll_save_results(&self, doc: &mut Document, error_list: &mut Vec<String>) {
        while let Ok(result) = self.save_result_receiver.try_recv() {
            match result {
                SaveResult::Autosave => {
                    doc.dirty_since_last_autosave = false;
                }
                SaveResult::ManualSave(path) => {
                    doc.savefile_path = path.display().to_string();
                    doc.canvas.render_next_frame = true;
                }
                SaveResult::Failed(msg) => {
                    error_list.push(format!("Save failed: {msg}"));
                }
            }
        }
    }
}
