//! File-dialog state machine: queues native dialogs, dispatches results.
//!
//! Owns the [`PendingFileAction`] / dialog channel pair and uses
//! [`DispatchedAction`] to decouple dialog results from the subsystem that
//! acts on them.

use std::path::PathBuf;
use std::sync::mpsc;

use eframe::egui::Color32;

use crate::app::ARCHIVE_EXTENSION;
use crate::app::ARCHIVE_FILTER_NAME;
use crate::app::CANVAS_EXTENSION;
use crate::app::DEFAULT_ARCHIVE_NAME;
use crate::app::DEFAULT_CANVAS_NAME;
use crate::app::EXPORT_FORMATS;
use crate::app::FILE_FILTER_NAME;
use crate::app::IMPORT_EXTENSIONS;
use crate::tools::brush_parsers::BrushTip;

/// A file-dialog action queued for execution on a background thread.
/// The result is received via channel at the start of a future frame.
#[derive(Clone, Copy, Debug)]
pub enum PendingFileAction {
    /// Open a native "load" dialog and deserialise a `.splattercanvas` file.
    Load,
    /// Open a native "save" dialog and serialise the current canvas.
    Save,
    /// Open a native "open" dialog for importing an image as a new canvas.
    Import,
    /// Open a native "save" dialog for exporting to one of the supported image
    /// formats. The `usize` payload indexes into [`EXPORT_FORMATS`].
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
    /// User cancelled the dialog.
    Cancelled,
}

/// An action decoded from a dialog result that the frame loop must dispatch
/// to the appropriate subsystem (save, load, import, or export).
pub enum DispatchedAction {
    /// Save the canvas to the given path.
    Save(PathBuf),
    /// Load a `.splattercanvas` file from the given path.
    Load(PathBuf),
    /// Import an image file as a new canvas.
    Import(PathBuf),
    /// Export the canvas as an image. The `usize` indexes into
    /// [`EXPORT_FORMATS`], the `PathBuf` is the destination.
    Export(usize, PathBuf),
    /// Serialise and export a `.splatterarchive` file.
    ExportArchive(PathBuf),
    /// Import a `.splatterarchive` file.
    ImportArchive(PathBuf),
}

/// Manages native file dialogs on background threads via `rfd`.
///
/// Owns the dialog-channel pair, the pending-action state machine, and
/// temporary storage for loaded stamp/brush data that the UI consumes.
pub struct DialogManager {
    /// File action queued for the next background thread iteration.
    pub pending_file_action: Option<PendingFileAction>,
    /// Channel sender for dispatching dialog requests to the background thread.
    pub dialog_sender: mpsc::Sender<DialogResult>,
    /// Channel receiver for receiving dialog results on the UI thread.
    pub dialog_receiver: mpsc::Receiver<DialogResult>,
    /// Result of a stamp-image load, consumed by the app frame after polling.
    pub loaded_stamp_data: Option<(Vec<Color32>, u32, u32, String)>,
    /// Result of a brush-file load, consumed by the app frame after polling.
    pub loaded_brush_data: Option<Vec<BrushTip>>,
}

impl std::fmt::Debug for DialogManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DialogManager")
            .field("pending_file_action", &self.pending_file_action)
            .field("has_loaded_stamp_data", &self.loaded_stamp_data.is_some())
            .field("has_loaded_brush_data", &self.loaded_brush_data.is_some())
            .finish()
    }
}

impl DialogManager {
    /// Create a new `DialogManager` with an open channel pair.
    ///
    /// # Parameters
    ///
    /// * `dialog_sender` — Channel sender for file-dialog results.
    /// * `dialog_receiver` — Channel receiver for file-dialog results.
    pub fn new(
        dialog_sender: mpsc::Sender<DialogResult>,
        dialog_receiver: mpsc::Receiver<DialogResult>,
    ) -> Self {
        Self {
            pending_file_action: None,
            dialog_sender,
            dialog_receiver,
            loaded_stamp_data: None,
            loaded_brush_data: None,
        }
    }

    /// Queue a file-dialog action and spawn it on a background thread.
    ///
    /// The dialog is dispatched to the main thread via `rfd` to avoid macOS
    /// winit re-entrancy panics.
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
                                        pixel[0],
                                        pixel[1],
                                        pixel[2],
                                        pixel[3],
                                    );
                                    pixels.push(straight);
                                }
                                let name = path
                                    .file_stem()
                                    .map(|s| s.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| "stamp".to_string());
                                let _ = sender
                                    .send(DialogResult::StampPixels(pixels, w, h, name));
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
    /// Reuses the existing [`PendingFileAction::Load`] handler by sending
    /// a synthetic `Picked` result through the dialog channel.
    ///
    /// # Parameters
    ///
    /// * `path` — The file path to load.
    pub fn queue_load_direct(&mut self, path: PathBuf) {
        self.pending_file_action = Some(PendingFileAction::Load);
        let _ = self.dialog_sender.send(DialogResult::Picked(path));
    }

    /// Drain the dialog result channel, handle simple results internally,
    /// and return a list of path-based actions for the caller to dispatch.
    ///
    /// Handles `StampPixels`, `BrushTips`, `Cancelled`, and `Error` internally.
    /// For `Picked` results, matches against `pending_file_action`, normalises
    /// the path (appending extension if needed), and returns the corresponding
    /// [`DispatchedAction`].
    ///
    /// # Parameters
    ///
    /// * `error_list` — Error list to push failure messages into.
    pub fn poll_dialog_results(
        &mut self,
        error_list: &mut Vec<String>,
    ) -> Vec<DispatchedAction> {
        let mut actions = Vec::new();
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
                    let action = match pending {
                        PendingFileAction::Save => {
                            let path_string = path.display().to_string();
                            let save_path =
                                if path_string.ends_with(CANVAS_EXTENSION) {
                                    path
                                } else {
                                    PathBuf::from(format!(
                                        "{path_string}{CANVAS_EXTENSION}"
                                    ))
                                };
                            Some(DispatchedAction::Save(save_path))
                        }
                        PendingFileAction::Load => {
                            Some(DispatchedAction::Load(path))
                        }
                        PendingFileAction::Import => {
                            Some(DispatchedAction::Import(path))
                        }
                        PendingFileAction::Export(index) => {
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
                            Some(DispatchedAction::Export(
                                index,
                                PathBuf::from(&path_string),
                            ))
                        }
                        PendingFileAction::ExportArchive => {
                            let path_string = path.display().to_string();
                            let final_path =
                                if path_string.ends_with(ARCHIVE_EXTENSION) {
                                    path
                                } else {
                                    PathBuf::from(format!(
                                        "{path_string}{ARCHIVE_EXTENSION}"
                                    ))
                                };
                            Some(DispatchedAction::ExportArchive(final_path))
                        }
                        PendingFileAction::ImportArchive => {
                            Some(DispatchedAction::ImportArchive(path))
                        }
                        PendingFileAction::LoadStamp | PendingFileAction::LoadBrush => {
                            None
                        }
                    };
                    if let Some(a) = action {
                        actions.push(a);
                    }
                }
            }
        }
        actions
    }
}
