//! Async file-dialog and save handling via mpsc channels.  Manages save-to-
//! current-path, save-as, load, and autosave workflows with result polling.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;

use chrono::Local;
use eframe::egui::Color32;

use crate::app::ARCHIVE_EXTENSION;
use crate::app::ARCHIVE_FILTER_NAME;
use crate::app::CANVAS_EXTENSION;
use crate::app::DEFAULT_ARCHIVE_NAME;
use crate::app::DEFAULT_CANVAS_NAME;
use crate::app::EXPORT_FORMATS;
use crate::app::FILE_FILTER_NAME;
use crate::app::IMPORT_EXTENSIONS;
use crate::canvas::Canvas;
use crate::canvas::Layer;
use crate::document::Document;
use crate::files::ExportStrategy;
use crate::tools::brush_parsers::BrushTip;
use crate::undo_history::UndoHistory;

// --- Autosave constants ---
const AUTOSAVE_DIRECTORY: &str = "autosaves";
const AUTOSAVE_DATE_FORMAT: &str = "%Y-%m-%d_%H-%M-%S";

// --- File-dialog types ---

/// A file-dialog action queued for execution on a background thread.
/// The result is received via channel at the start of a future frame.
#[derive(Clone, Copy, Debug)]
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
    /// Open a native "save" dialog and export as xz-compressed `.splatterarchive`.
    ExportArchive,
    /// Open a native "open" dialog and import a `.splatterarchive` file.
    ImportArchive,
}

/// Message sent back from the file-dialog thread to the UI thread.
pub enum DialogResult {
    /// A file path selected by the user in an open/save dialog.
    Picked(PathBuf),
    /// Decoded stamp image pixels + dimensions + suggested name (file stem).
    StampPixels(Vec<Color32>, u32, u32, String),
    /// Parsed brush tips from an ABR/GBR file.
    BrushTips(Vec<BrushTip>),
    /// An error occurred during a file operation.
    Error(String),
    /// User cancelled the dialog — clears `pending_file_action`.
    Cancelled,
}

/// Distinguishes an autosave from a manual save in the async save pipeline.
pub enum SaveKind {
    /// Periodic autosave to `{data_dir}/autosaves/`.
    Autosave,
    /// Explicit user-initiated save to a chosen path.
    ManualSave(PathBuf),
}

/// Result of an async load or import operation sent via channel.
///
/// Uses `Vec<Layer>` + dimensions instead of `Canvas` directly so the data
/// is [`Send`] (avoids `Canvas`'s non-Send `TextureHandle` field). The UI
/// thread reconstructs a `Canvas` from the layers when the result is polled.
pub enum LoadImportResult {
    /// Canvas loaded from a `.splattercanvas` file, plus the source path.
    Loaded(Canvas, String),
    /// Image imported as a new canvas.
    Imported(Vec<Layer>, u32, u32),
    /// Canvas imported from a `.splatterarchive` file.
    ArchiveImported(Canvas),
    /// Operation failed with an error message.
    Failed(String),
}

/// Result sent back via channel when an async save completes.
#[derive(Debug)]
pub enum SaveResult {
    /// Autosave completed successfully (resulting path is not surfaced).
    Autosave,
    /// Manual save completed to the given path.
    ManualSave(PathBuf),
    /// Archive autosave completed successfully (path not surfaced).
    ArchiveAutosave,
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
    /// Injected export strategy for writing image files.
    ///
    /// Defaults to [`DefaultExportStrategy`](crate::files::DefaultExportStrategy)
    /// which handles all 13 supported formats.
    pub export_strategy: std::sync::Arc<dyn ExportStrategy + Send + Sync>,
    /// Channel sender for export results from background thread.
    pub export_result_sender: mpsc::Sender<anyhow::Result<()>>,
    /// Channel receiver for export results on the UI thread.
    pub export_result_receiver: mpsc::Receiver<anyhow::Result<()>>,
    /// `true` while an async export thread is running.
    pub export_in_flight: bool,
    /// Channel sender for load/import results from background thread.
    pub load_import_sender: mpsc::Sender<LoadImportResult>,
    /// Channel receiver for load/import results on the UI thread.
    pub load_import_receiver: mpsc::Receiver<LoadImportResult>,
    /// `true` while an async load thread is running.
    pub load_in_flight: bool,
    /// `true` while an async import thread is running.
    pub import_in_flight: bool,
    /// `true` when the most recently triggered async save is an autosave.
    /// Used by the UI to display "Autosaving…" vs "Saving…" in the status bar.
    pub autosave_in_flight: bool,
    /// `true` while an archive autosave (`.splatterarchive`) is in flight.
    pub archive_autosave_in_flight: bool,
}

impl std::fmt::Debug for FileIO {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileIO")
            .field("pending_file_action", &self.pending_file_action)
            .field("app_local_data_directory", &self.app_local_data_directory)
            .field("has_loaded_stamp_data", &self.loaded_stamp_data.is_some())
            .field("has_loaded_brush_data", &self.loaded_brush_data.is_some())
            .field("export_in_flight", &self.export_in_flight)
            .field("load_in_flight", &self.load_in_flight)
            .field("import_in_flight", &self.import_in_flight)
            .field("autosave_in_flight", &self.autosave_in_flight)
            .finish()
    }
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
    /// * `export_strategy` — Image export implementation.
    pub fn new(
        dialog_sender: mpsc::Sender<DialogResult>,
        dialog_receiver: mpsc::Receiver<DialogResult>,
        save_result_sender: mpsc::Sender<SaveResult>,
        save_result_receiver: mpsc::Receiver<SaveResult>,
        app_local_data_directory: PathBuf,
        export_strategy: std::sync::Arc<dyn ExportStrategy + Send + Sync>,
    ) -> Self {
        let (export_result_sender, export_result_receiver) = mpsc::channel();
        let (load_import_sender, load_import_receiver) = mpsc::channel();
        Self {
            pending_file_action: None,
            dialog_sender,
            dialog_receiver,
            save_result_sender,
            save_result_receiver,
            export_result_sender,
            export_result_receiver,
            export_in_flight: false,
            load_import_sender,
            load_import_receiver,
            load_in_flight: false,
            import_in_flight: false,
            autosave_in_flight: false,
            archive_autosave_in_flight: false,
            app_local_data_directory,
            loaded_stamp_data: None,
            loaded_brush_data: None,
            export_strategy,
        }
    }

    /// Return the path to the autosave directory (`{data_dir}/autosaves/`).
    ///
    /// The directory is created during app startup and may contain multiple
    /// timestamped `.splattercanvas` files.
    pub fn autosave_directory(&self) -> PathBuf {
        self.app_local_data_directory.join(AUTOSAVE_DIRECTORY)
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

        #[inline]
        fn send_cancelled(sender: &mpsc::Sender<DialogResult>) {
            let _ = sender.send(DialogResult::Cancelled);
        }

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
                    } else {
                        send_cancelled(&sender);
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
                    } else {
                        send_cancelled(&sender);
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
                    } else {
                        send_cancelled(&sender);
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
                    } else {
                        send_cancelled(&sender);
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
                    } else {
                        send_cancelled(&sender);
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
                    } else {
                        send_cancelled(&sender);
                    }
                });
            }
            PendingFileAction::ExportArchive => {
                self.pending_file_action = Some(PendingFileAction::ExportArchive);
                std::thread::spawn(move || {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(
                            ARCHIVE_FILTER_NAME,
                            &[ARCHIVE_EXTENSION.trim_start_matches('.')],
                        )
                        .set_file_name(DEFAULT_ARCHIVE_NAME)
                        .save_file()
                    {
                        let _ = sender.send(DialogResult::Picked(path));
                    } else {
                        send_cancelled(&sender);
                    }
                });
            }
            PendingFileAction::ImportArchive => {
                self.pending_file_action = Some(PendingFileAction::ImportArchive);
                std::thread::spawn(move || {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(
                            ARCHIVE_FILTER_NAME,
                            &[ARCHIVE_EXTENSION.trim_start_matches('.')],
                        )
                        .pick_file()
                    {
                        let _ = sender.send(DialogResult::Picked(path));
                    } else {
                        send_cancelled(&sender);
                    }
                });
            }
        }
    }

    /// Queue a direct file load without showing a dialog.
    ///
    /// Reuses the existing `PendingFileAction::Load` handler by sending
    /// a synthetic `Picked` result through the dialog channel.
    ///
    /// # Parameters
    ///
    /// * `path` — The file path to load.
    pub fn queue_load_direct(&mut self, path: PathBuf) {
        self.pending_file_action = Some(PendingFileAction::Load);
        let _ = self.dialog_sender.send(DialogResult::Picked(path));
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
                DialogResult::Cancelled => {
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
                        PendingFileAction::Load => {
                            self.trigger_async_load(path);
                        }
                        PendingFileAction::Import => {
                            self.trigger_async_import(path);
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
                            let rgba = Arc::clone(&document.canvas.output_rgba);
                            let w = document.canvas.width;
                            let h = document.canvas.height;
                            let export_path = PathBuf::from(&path_string);
                            self.trigger_async_export(rgba, w, h, export_path);
                        }
                        PendingFileAction::LoadStamp => {
                            // Handled exclusively by the StampPixels variant above.
                        }
                        PendingFileAction::LoadBrush => {
                            // Handled exclusively by the BrushTips variant above.
                        }
                        PendingFileAction::ExportArchive => {
                            let path_string = path.display().to_string();
                            let final_path = if path_string.ends_with(ARCHIVE_EXTENSION) {
                                path
                            } else {
                                PathBuf::from(format!("{path_string}{ARCHIVE_EXTENSION}"))
                            };
                            let canvas = &*document.canvas;
                            self.trigger_async_export_archive(canvas.clone(), final_path);
                        }
                        PendingFileAction::ImportArchive => {
                            self.trigger_async_import_archive(path);
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
    /// via `save_result_sender`. Sets [`SaveState::InFlight`] on the document
    /// while the save thread runs.
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

    /// Spawn a background thread to serialize and write a timestamped
    /// `.splatterarchive` autosave file under `AUTOSAVE_DIRECTORY`.
    ///
    /// Results are sent via [`save_result_sender`] as [`SaveResult::ArchiveAutosave`]
    /// (or `Failed`). The archive autosave is a separate stream from the regular
    /// `.splattercanvas` autosave.
    ///
    /// # Parameters
    ///
    /// * `document` — The document whose canvas will be archived.
    pub fn trigger_async_autosave_archive(&mut self, document: &Document) {
        self.archive_autosave_in_flight = true;
        let canvas = document.canvas.clone();
        let path = self
            .app_local_data_directory
            .join(AUTOSAVE_DIRECTORY)
            .join(format!(
                "{}{}",
                Local::now().format(AUTOSAVE_DATE_FORMAT),
                ARCHIVE_EXTENSION,
            ));
        let sender = self.save_result_sender.clone();
        std::thread::spawn(move || {
            let result = match crate::files::save_canvas_to_path_xz(&canvas, &path) {
                Ok(()) => SaveResult::ArchiveAutosave,
                Err(error) => SaveResult::Failed(format!("Archive autosave failed: {error}")),
            };
            let _ = sender.send(result);
        });
    }

    /// Spawn a background thread to encode and write the exported image.
    ///
    /// Clones the blended RGBA buffer to avoid holding a reference across
    /// the thread boundary. Sends the result (success or error string) back
    /// via `export_result_receiver`. The caller should call
    /// [`poll_export_results`](Self::poll_export_results) next frame.
    ///
    /// # Parameters
    ///
    /// * `premultiplied_rgba` — The already-blended premultiplied RGBA buffer.
    /// * `width` — Image width in pixels.
    /// * `height` — Image height in pixels.
    /// * `path` — Destination file path.
    pub fn trigger_async_export(
        &mut self,
        premultiplied_rgba: Arc<Vec<u8>>,
        width: u32,
        height: u32,
        path: PathBuf,
    ) {
        self.export_in_flight = true;
        let strategy = std::sync::Arc::clone(&self.export_strategy);
        let sender = self.export_result_sender.clone();
        std::thread::spawn(move || {
            let result = strategy.export(&premultiplied_rgba, width, height, &path);
            let _ = sender.send(result);
        });
    }

    /// Spawn a background thread to read and deserialize a `.splattercanvas` file.
    ///
    /// Sends the result (layers, dimensions, path) via the load/import channel.
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
    /// Sends the result (layers, dimensions) via the load/import channel.
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

    /// Spawn a background thread to serialize and write an xz-compressed
    /// `.splatterarchive` file (one-shot archive export).
    ///
    /// Takes an already-cloned [`Canvas`] (cheap [`Arc`] clone) so the UI
    /// thread can continue drawing while compression runs on a background
    /// thread. Sends the result via [`export_result_sender`]; poll with
    /// [`poll_export_results`](Self::poll_export_results).
    ///
    /// # Parameters
    ///
    /// * `canvas` — The canvas to serialize (cloned from [`Document`]).
    /// * `path` — Destination file path for the `.splatterarchive` file.
    pub fn trigger_async_export_archive(&mut self, canvas: Canvas, path: PathBuf) {
        self.export_in_flight = true;
        let sender = self.export_result_sender.clone();
        std::thread::spawn(move || {
            let result = crate::files::save_canvas_to_path_xz(&canvas, &path);
            let _ = sender.send(result);
        });
    }

    /// Spawn a background thread to read and deserialize a `.splatterarchive` file.
    ///
    /// Sends the result via the load/import channel.
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
                        output_rgba: Arc::new(Vec::new()),
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
                    // Don't know if this was load or import; clear both flags.
                    self.load_in_flight = false;
                    self.import_in_flight = false;
                    error_list.push(message);
                }
            }
        }
    }

    /// Poll for completed async export results.
    ///
    /// Pushes errors to `error_list`. Returns `true` if an export result was
    /// processed (either success or failure).
    ///
    /// # Parameters
    ///
    /// * `error_list` — Error list to push failure messages into.
    pub fn poll_export_results(&mut self, error_list: &mut Vec<String>) -> bool {
        if let Ok(result) = self.export_result_receiver.try_recv() {
            self.export_in_flight = false;
            if let Err(error) = result {
                error_list.push(format!("Export failed: {error}"));
            }
            true
        } else {
            false
        }
    }

    /// Poll for completed async save results and update state accordingly.
    ///
    /// Marks the document as clean after autosave, sets the save path
    /// after manual save, pushes errors to the error list, and
    /// resets [`SaveState`] to `Idle` when results are consumed.
    ///
    /// # Parameters
    ///
    /// * `document` — The document to update save-path / dirty-flag on.
    /// * `error_list` — Error list to push failure messages into.
    pub fn poll_save_results(&mut self, document: &mut Document, error_list: &mut Vec<String>) {
        let mut had_regular_result = false;
        let mut had_archive_autosave = false;
        while let Ok(result) = self.save_result_receiver.try_recv() {
            match result {
                SaveResult::Autosave => {
                    had_regular_result = true;
                    document.dirty_since_last_autosave = false;
                }
                SaveResult::ManualSave(path) => {
                    had_regular_result = true;
                    document.savefile_path = path.display().to_string();
                    document.dirty_since_last_autosave = false;
                    document.canvas_mut().dirty_rect.request_full_blend();
                }
                SaveResult::ArchiveAutosave => {
                    had_archive_autosave = true;
                }
                SaveResult::Failed(message) => {
                    error_list.push(format!("Save failed: {message}"));
                    // Could be from either stream; clear both to be safe.
                    had_regular_result = true;
                    had_archive_autosave = true;
                }
            }
        }
        if had_regular_result {
            document.save_state = crate::document::SaveState::Idle;
            self.autosave_in_flight = false;
        }
        if had_archive_autosave {
            self.archive_autosave_in_flight = false;
        }
    }
}
