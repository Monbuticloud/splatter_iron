//! Top-level application state, identity constants, export formats, autosave
//! loop, and wiring between the document, tools, undo history, and file IO.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use directories::ProjectDirs;
use eframe::egui::Panel;
use eframe::egui::{ self };
use serde::Deserialize;
use serde::Serialize;
use eframe::egui_wgpu::wgpu;
use crate::debug::debug_snapshot;

use crate::asset_library::Library;
use crate::brush_library::BrushEntry;
use crate::canvas::Canvas;
use crate::canvas::CurrentTool;
use crate::canvas::RenderState;
use crate::document::Document;
use crate::file_io::FileIO;
use crate::file_io::PendingFileAction;
use crate::stamp_library::StampEntry;
use crate::tool_configuration::ToolConfiguration;
use crate::undo_history::UndoHistory;

pub mod frame;

// --- App identity constants ---
/// Reverse-domain qualifier for the platform data directory.
pub const APP_QUALIFIER: &str = "com";
/// Organization name for the platform data directory.
pub const APP_ORGANIZATION: &str = "Monbuticloud";
/// Application name for the platform data directory and window title.
pub const APP_NAME: &str = "SplatterIron";

/// File extension for native canvas files (zstd-compressed JSON).
pub const CANVAS_EXTENSION: &str = ".splattercanvas";
/// File-dialog filter name for `.splattercanvas` files.
pub const FILE_FILTER_NAME: &str = "SplatterCanvas";
/// Default save-file name used when no path has been set.
pub const DEFAULT_CANVAS_NAME: &str = "canvas.splattercanvas";

/// Preset canvas sizes shown in the "New Canvas" dialog.
pub(crate) const NEW_CANVAS_PRESETS: &[(&str, u32, u32)] = &[
    ("XS", 800, 600),
    ("S", 1280, 960),
    ("M", 2000, 1500),
    ("L", 2560, 1920),
    ("XL", 3200, 2400),
];

// --- Performance constants ---
pub(crate) const UNFOCUSED_SLEEP_MILLISECONDS: u64 = 50;

// --- Memory warning threshold ---
/// Threshold (in bytes) above which creating a new canvas shows a
/// confirmation dialog. 500 MB is a safe boundary — output_rgba + one layer
/// + blend buffer at 8000×8000 is ~768 MB.
pub(crate) const MEMORY_WARNING_THRESHOLD: u64 = 500_000_000;

/// Estimate the minimum memory footprint (bytes) for a canvas of the given
/// dimensions: output_rgba (w×h×4) + one layer (w×h×4) + blend buffer
/// overhead (w×h×4). The actual footprint is higher with multiple layers.
pub(crate) fn estimate_canvas_memory(width: u32, height: u32) -> u64 {
    let pixels = u64::from(width) * u64::from(height);
    pixels * 12
}
pub(crate) const REPAINT_DELAY_MULTIPLIER: u32 = 5;

// --- Autosave interval ---
pub(crate) const AUTOSAVE_INTERVAL_MINUTES: u64 = 2;

// --- Image import extensions ---
/// File-extension list accepted by the image-import dialog (19 formats).
/// Serialization wrapper for tool config + recent files.
#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedConfig {
    /// Tool settings (current tool, color, radius, etc.).
    pub tool_configuration: ToolConfiguration,
    /// Recently opened/saved file paths.
    pub recent_files: Vec<PathBuf>,
}

pub const IMPORT_EXTENSIONS: &[&str] = &[
    "avif",
    "png",
    "jpg",
    "jpeg",
    "webp",
    "gif",
    "tiff",
    "tif",
    "tga",
    "ico",
    "pnm",
    "pgm",
    "ppm",
    "pbm",
    "pam",
    "qoi",
    "exr",
    "hdr",
    "ff",
];

/// File extension list and image format for an export target.
#[derive(Debug)]
pub struct ExportInformation {
    pub extensions: &'static [&'static str],
    #[allow(dead_code)]
    pub fmt: image::ImageFormat,
}

/// Lookup table for all supported export formats.
pub const EXPORT_FORMATS: &[(&str, ExportInformation)] = &[
    (
        "AVIF",
        ExportInformation {
            extensions: &["avif"],
            fmt: image::ImageFormat::Avif,
        },
    ),
    (
        "PNG",
        ExportInformation {
            extensions: &["png"],
            fmt: image::ImageFormat::Png,
        },
    ),
    (
        "JPEG",
        ExportInformation {
            extensions: &["jpg", "jpeg"],
            fmt: image::ImageFormat::Jpeg,
        },
    ),
    (
        "WebP",
        ExportInformation {
            extensions: &["webp"],
            fmt: image::ImageFormat::WebP,
        },
    ),
    (
        "GIF",
        ExportInformation {
            extensions: &["gif"],
            fmt: image::ImageFormat::Gif,
        },
    ),
    (
        "TIFF",
        ExportInformation {
            extensions: &["tiff", "tif"],
            fmt: image::ImageFormat::Tiff,
        },
    ),
    (
        "TGA",
        ExportInformation {
            extensions: &["tga"],
            fmt: image::ImageFormat::Tga,
        },
    ),
    (
        "ICO",
        ExportInformation {
            extensions: &["ico"],
            fmt: image::ImageFormat::Ico,
        },
    ),
    (
        "PNM",
        ExportInformation {
            extensions: &["pnm", "pgm", "ppm", "pbm", "pam"],
            fmt: image::ImageFormat::Pnm,
        },
    ),
    (
        "QOI",
        ExportInformation {
            extensions: &["qoi"],
            fmt: image::ImageFormat::Qoi,
        },
    ),
    (
        "EXR",
        ExportInformation {
            extensions: &["exr"],
            fmt: image::ImageFormat::OpenExr,
        },
    ),
    (
        "HDR",
        ExportInformation {
            extensions: &["hdr"],
            fmt: image::ImageFormat::Hdr,
        },
    ),
    (
        "Farbfeld",
        ExportInformation {
            extensions: &["ff"],
            fmt: image::ImageFormat::Farbfeld,
        },
    ),
];

/// A stamp image awaiting a user-provided name before being added to the library.
pub struct PendingStamp {
    pub pixels: Vec<egui::Color32>,
    pub width: u32,
    pub height: u32,
    pub name: String,
    /// Spacing percentage (0–100) — used when this is a brush tip, ignored for stamps.
    pub spacing: u8,
}

impl std::fmt::Debug for PendingStamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingStamp")
            .field("pixels.len", &self.pixels.len())
            .field("width", &self.width)
            .field("height", &self.height)
            .field("name", &self.name)
            .field("spacing", &self.spacing)
            .finish()
    }
}

/// Dialog-related state: open/closed flags, input values, pending confirmations.
#[derive(Debug)]
pub struct DialogState {
    /// Layer index pending deletion confirmation via modal dialog.
    pub show_delete_layer_dialog: Option<usize>,
    /// Whether the "New Canvas" dialog is currently open.
    pub show_new_canvas_dialog: bool,
    /// Width input for the new canvas dialog (in pixels).
    pub new_canvas_width: u32,
    /// Height input for the new canvas dialog (in pixels).
    pub new_canvas_height: u32,
    /// Dimensions of a canvas pending user confirmation because it exceeds the
    /// memory warning threshold. `None` when no confirmation is pending.
    pub pending_large_canvas: Option<(u32, u32)>,
    /// A stamp image awaiting a name from the user before being added to the library.
    pub pending_stamp_name: Option<PendingStamp>,
    /// Brush tips awaiting user-confirmed names before being added to the library.
    pub pending_brushes: Option<Vec<PendingStamp>>,
    /// A destructive action waiting for the user to resolve unsaved changes.
    pub pending_unsaved_action: Option<UnsavedWarningAction>,
    /// A destructive action that was deferred until the current save completes.
    pub pending_after_save: Option<UnsavedWarningAction>,
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            show_delete_layer_dialog: None,
            show_new_canvas_dialog: false,
            new_canvas_width: 2000,
            new_canvas_height: 1500,
            pending_large_canvas: None,
            pending_stamp_name: None,
            pending_brushes: None,
            pending_unsaved_action: None,
            pending_after_save: None,
        }
    }
}

/// Error messages displayed in the error overlay.
#[derive(Debug)]
pub struct ErrorState {
    pub list: Vec<String>,
}

impl Default for ErrorState {
    fn default() -> Self {
        Self { list: Vec::new() }
    }
}

/// Transient toast notification.
#[derive(Debug)]
pub struct ToastState {
    /// The message text and the instant it was triggered.
    pub message: Option<(String, Instant)>,
}

impl Default for ToastState {
    fn default() -> Self {
        Self { message: None }
    }
}

/// A destructive file action that was postponed because the canvas has
/// unsaved changes. Stored until the user resolves the unsaved-changes
/// warning dialog.
#[derive(Clone, Debug)]
pub enum UnsavedWarningAction {
    /// Close the application.
    Quit,
    /// Open the new-canvas dialog.
    NewCanvas,
    /// Load a `.splattercanvas` file (opens file dialog).
    Load,
    /// Import an image as a new canvas (opens file dialog).
    Import,
    /// Load a specific recent `.splattercanvas` file by path.
    LoadPath(std::path::PathBuf),
}

/// Progress state for long-running operations.
#[derive(Clone, Debug, PartialEq)]
pub enum ProgressState {
    /// No operation in progress.
    Idle,
    /// Exporting an image file.
    Exporting,
    /// Loading a `.splattercanvas` file.
    Loading,
    /// Importing an image file as a new canvas.
    Importing,
}

/// UI-level state that doesn't belong to any domain module.
#[derive(Debug)]
pub struct UIState {
    /// Current rendering cadence (active, throttled, or frozen).
    pub render_state: RenderState,
    /// Total time elapsed since app start.
    pub time_elapsed: Duration,
    /// Number of autosaves performed this session.
    pub times_autosaved: u32,
    /// Wall-clock time when the last autosave completed.
    pub last_autosave_time: Duration,
    /// Tool selected before the current one (used for eraser toggle-back).
    pub previous_tool: Option<CurrentTool>,
    /// Cursor position from the previous frame (used for brush preview).
    pub previous_cursor_position: Option<(u32, u32)>,
    /// Multiplier applied to undo/redo step count during fast-scroll.
    pub undo_redo_steps_multiplier: usize,
    /// Maximum 2D texture dimension supported by the GPU device.
    /// Queried from `device.limits().max_texture_dimension_2d`; falls back
    /// to 8192 (WebGPU minimum) when the wgpu backend is unavailable.
    pub max_texture_dimension: u32,
    /// Recently saved/loaded files (most recent first, max 10).
    pub recent_files: Vec<PathBuf>,
    /// Index of the last-used export format in [`EXPORT_FORMATS`].
    pub last_export_format: usize,
    /// Cached window title — updated when dirty flag changes.
    pub current_title: String,
    /// Dialog-related state.
    pub dialogs: DialogState,
    /// Error messages displayed in the error overlay.
    pub errors: ErrorState,
    /// Transient toast notification.
    pub toasts: ToastState,
    /// Long-running operation progress.
    pub progress: ProgressState,
    /// Set to `true` when the app should close (e.g., after unsaved-changes
    /// resolution chooses Quit).
    pub should_close: bool,
    /// Pan offset from center, in screen pixels.
    pub pan_offset: egui::Vec2,
    /// Current zoom level (1.0 = fit to screen).
    pub zoom: f32,
}

impl Default for UIState {
    /// Create a default `UIState` with idle throttled rendering,
    /// zero elapsed time, no autosaves, and no pending layer deletion.
    fn default() -> Self {
        Self {
            render_state: RenderState::IdleThrottled,
            time_elapsed: Duration::ZERO,
            times_autosaved: 0,
            last_autosave_time: Duration::ZERO,
            max_texture_dimension: 8192,
            previous_tool: None,
            previous_cursor_position: None,
            undo_redo_steps_multiplier: 1,
            recent_files: Vec::new(),
            last_export_format: 1,
            current_title: APP_NAME.to_string(),
            dialogs: DialogState::default(),
            errors: ErrorState::default(),
            toasts: ToastState::default(),
            progress: ProgressState::Idle,
            should_close: false,
            pan_offset: egui::Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

/// WGPU GPU texture and associated state for partial canvas uploads.
///
/// Created once during `MyApp` construction and updated on canvas resize.
/// The `queue` is used each frame to upload only the dirty sub-region
/// via `wgpu::Queue::write_texture`.
#[derive(Debug)]
pub struct GpuTexture {
    /// The wgpu texture storing the canvas composite on the GPU.
    pub texture: wgpu::Texture,
    /// The egui texture ID registered with the egui_wgpu renderer for display.
    pub texture_id: egui::TextureId,
    /// The wgpu queue used for uploading dirty-rect data to the GPU.
    pub queue: Arc<wgpu::Queue>,
}

/// Top-level application state owned by eframe: document, tools, undo history,
/// file IO, UI state, and optional wgpu GPU texture.
#[derive(Debug)]
pub struct MyApp {
    /// The edited canvas document (layers, dimensions, save path).
    pub document: Document,
    /// Active tool configuration (tool, color, radius, alpha overlay).
    pub tool_configuration: ToolConfiguration,
    /// Undo/redo history stack with visited-stamp deduplication.
    pub undo: UndoHistory,
    /// Async file dialog and save operation manager.
    pub file_io: FileIO,
    /// UI render state, autosave tracking, and dialog flags.
    pub ui: UIState,
    /// GPU texture for partial-upload rendering.
    ///
    /// `Some` when the wgpu backend is available; `None` falls back to
    /// the egui-managed texture path (full-buffer `tex.set()`).
    pub gpu_texture: Option<GpuTexture>,
    /// Persistent stamp library (images, naming, disk storage).
    pub stamp_library: Library<StampEntry>,
    /// Persistent custom brush library (imported tips, naming, disk storage).
    pub brush_library: Library<BrushEntry>,
}

impl MyApp {
    /// Create a new `MyApp`, initializing the wgpu GPU texture for partial uploads.
    ///
    /// Falls back to the egui-managed texture path (full-buffer `tex.set()`)
    /// when wgpu render state is unavailable (e.g. Glow backend).
    ///
    /// # Panics
    ///
    /// Panics if the platform-specific data directory cannot be resolved
    /// (no home directory) or if the operating system refuses to create
    /// either the data directory or the autosaves subdirectory (e.g.,
    /// file-system permissions).
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        use std::sync::mpsc;
        let (dialog_sender, dialog_receiver) = mpsc::channel();
        let (save_result_sender, save_result_receiver) = mpsc::channel();
        let canvas = Canvas::default();
        let pixel_count = (canvas.width * canvas.height) as usize;

        let project_dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME).expect(
            "Couldn't resolve app dir"
        );
        let data_dir = project_dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&data_dir).expect("Couldn't create data dir");
        std::fs::create_dir_all(&data_dir.join("autosaves")).expect("Couldn't create autosave dir");

        // Query the device's max 2D texture dimension for the new-canvas slider.
        // Falls back to 8192 (WebGPU minimum) when the wgpu backend is unavailable.
        let max_texture_dimension = creation_context.wgpu_render_state
            .as_ref()
            .map(|rs| rs.device.limits().max_texture_dimension_2d)
            .unwrap_or(8192);

        let stamp_library: Library<StampEntry> = Library::load_from_disk(&data_dir);
        let brush_library: Library<BrushEntry> = Library::load_from_disk(&data_dir);

        let gpu_texture = creation_context.wgpu_render_state.as_ref().map(|render_state| {
            let width = canvas.width;
            let height = canvas.height;
            let size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };
            let texture = render_state.device.create_texture(
                &(wgpu::TextureDescriptor {
                    label: Some("splatter_iron_canvas"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                })
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mut renderer = render_state.renderer.write();
            let texture_id = renderer.register_native_texture(
                &render_state.device,
                &view,
                wgpu::FilterMode::Linear
            );
            GpuTexture {
                texture,
                texture_id,
                queue: Arc::new(render_state.queue.clone()),
            }
        });

        let (tool_configuration, recent_files) = data_dir
            .join("config.json")
            .as_path()
            .try_exists()
            .ok()
            .filter(|&exists| exists)
            .and_then(|_| {
                std::fs::File
                    ::open(data_dir.join("config.json"))
                    .ok()
                    .and_then(|file| {
                        let p: PersistedConfig = serde_json::from_reader(file).ok()?;
                        Some((p.tool_configuration, p.recent_files))
                    })
            })
            .unwrap_or_default();

        Self {
            document: Document::new(canvas),
            tool_configuration,
            undo: UndoHistory::new(pixel_count),
            file_io: FileIO::new(
                dialog_sender,
                dialog_receiver,
                save_result_sender,
                save_result_receiver,
                data_dir,
                std::sync::Arc::new(crate::files::DefaultExportStrategy)
            ),
            ui: UIState {
                max_texture_dimension,
                recent_files,
                ..UIState::default()
            },
            gpu_texture,
            stamp_library,
            brush_library,
        }
    }

}

// --- Frame-helper methods (called once per frame from `ui()`) ---

impl MyApp {




    /// Render all four panels (top, left, right, centre) and return whether
    /// the user requested to quit (via the top panel).
    fn show_panels(&mut self, ui: &mut egui::Ui) -> bool {
        let is_quitting = Panel::top("top").show_inside(ui, |ui| self.show_top_panel(ui)).inner;

        Panel::left("side").show_inside(ui, |ui| self.show_left_panel(ui));
        Panel::right("right").show_inside(ui, |ui| self.show_right_panel(ui));
        egui::CentralPanel::default().show_inside(ui, |ui| self.show_central_panel(ui));

        is_quitting
    }

    /// Show the error-list window (dismiss, copy, dismiss-all).
    fn show_error_window(&mut self, ui: &mut egui::Ui) {
        if self.ui.errors.list.is_empty() {
            return;
        }
        let mut open = true;
        let mut to_dismiss: Vec<usize> = Vec::new();
        egui::Window
            ::new("Error")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                for (index, msg) in self.ui.errors.list.iter().enumerate() {
                    ui.label(format!("Error: {msg}"));
                    ui.horizontal(|ui| {
                        if ui.button("Dismiss").clicked() {
                            to_dismiss.push(index);
                        }
                        if ui.button("Copy error").clicked() {
                            ui.ctx().copy_text(msg.clone());
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("Dismiss All").clicked() {
                        to_dismiss.extend(0..self.ui.errors.list.len());
                    }
                });
            });

        to_dismiss.sort_unstable_by(|a, b| b.cmp(a));
        to_dismiss.dedup();
        for i in to_dismiss {
            self.ui.errors.list.remove(i);
        }
        if !open {
            self.ui.errors.list.clear();
        }
    }

    /// Show the "Large Canvas Warning" confirmation dialog.
    fn show_large_canvas_warning(&mut self, ui: &mut egui::Ui) {
        let Some((w, h)) = self.ui.dialogs.pending_large_canvas else {
            return;
        };
        let mut open = true;
        let estimated = estimate_canvas_memory(w, h);
        let estimated_mb = estimated / (1024 * 1024);
        egui::Window
            ::new("Large Canvas Warning")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.label(
                    format!(
                        "This canvas ({w}×{h}) may use up to ~{estimated_mb} MB of RAM.\n\
                     Proceed? This cannot be undone."
                    )
                );
                ui.horizontal(|ui| {
                    if ui.button("Yes, create").clicked() {
                        let canvas = Canvas::new(w, h);
                        self.document.replace_canvas(canvas, &mut self.undo);
                        self.ui.previous_tool = None;
                        self.ui.previous_cursor_position = None;
                        self.ui.dialogs.show_new_canvas_dialog = false;
                        self.ui.dialogs.pending_large_canvas = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.ui.dialogs.pending_large_canvas = None;
                    }
                });
            });
        if !open {
            self.ui.dialogs.pending_large_canvas = None;
        }
    }

    /// Show the "Delete Layer" confirmation dialog.
    fn show_delete_layer_dialog(&mut self, ui: &mut egui::Ui) {
        let Some(index) = self.ui.dialogs.show_delete_layer_dialog else {
            return;
        };
        let layer_name = self.document.canvas.pixels
            .get(index)
            .map(|l| {
                if l.name.is_empty() { format!("Layer {index}") } else { l.name.clone() }
            })
            .unwrap_or_default();
        let mut open = true;
        egui::Window
            ::new("Delete Layer")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.label(format!("Delete \"{layer_name}\"? This cannot be undone."));
                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        self.document.delete_layer(index, &mut self.undo);
                        self.ui.dialogs.show_delete_layer_dialog = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.ui.dialogs.show_delete_layer_dialog = None;
                    }
                });
            });
        if !open {
            self.ui.dialogs.show_delete_layer_dialog = None;
        }
    }

    /// Show the "New Canvas" preset / custom-size dialog.
    fn show_new_canvas_dialog(&mut self, ui: &mut egui::Ui) {
        if !self.ui.dialogs.show_new_canvas_dialog {
            return;
        }
        let mut open = true;
        egui::Window
            ::new("New Canvas")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for &(label, width, height) in NEW_CANVAS_PRESETS {
                        if ui.button(format!("{label}\n{width}×{height}")).clicked() {
                            self.ui.dialogs.new_canvas_width = width;
                            self.ui.dialogs.new_canvas_height = height;
                        }
                    }
                });
                ui.separator();
                ui.label("Custom:");
                ui.add(
                    egui::Slider
                        ::new(
                            &mut self.ui.dialogs.new_canvas_width,
                            4..=self.ui.max_texture_dimension
                        )
                        .text("Width")
                );
                ui.add(
                    egui::Slider
                        ::new(
                            &mut self.ui.dialogs.new_canvas_height,
                            4..=self.ui.max_texture_dimension
                        )
                        .text("Height")
                );
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        if self.document.dirty_since_last_autosave {
                            self.ui.dialogs.pending_unsaved_action = Some(
                                UnsavedWarningAction::NewCanvas
                            );
                        } else {
                            let mem = estimate_canvas_memory(
                                self.ui.dialogs.new_canvas_width,
                                self.ui.dialogs.new_canvas_height
                            );
                            if mem > MEMORY_WARNING_THRESHOLD {
                                self.ui.dialogs.pending_large_canvas = Some((
                                    self.ui.dialogs.new_canvas_width,
                                    self.ui.dialogs.new_canvas_height,
                                ));
                            } else {
                                let canvas = Canvas::new(
                                    self.ui.dialogs.new_canvas_width,
                                    self.ui.dialogs.new_canvas_height
                                );
                                self.document.replace_canvas(canvas, &mut self.undo);
                                self.ui.previous_tool = None;
                                self.ui.previous_cursor_position = None;
                                self.ui.dialogs.show_new_canvas_dialog = false;
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.ui.dialogs.show_new_canvas_dialog = false;
                    }
                });
            });
        if !open {
            self.ui.dialogs.show_new_canvas_dialog = false;
        }
    }

    /// Show the unsaved-changes warning when a destructive action is triggered
    /// while the canvas has unsaved modifications.
    ///
    /// Offers Save (save then proceed), Discard (lose changes and proceed),
    /// and Cancel (do nothing). The deferred action is stored in
    /// `pending_unsaved_action` and cleared on resolution.
    fn show_unsaved_changes_warning(&mut self, ui: &mut egui::Ui) {
        if self.ui.dialogs.pending_unsaved_action.is_none() {
            return;
        }
        let action = self.ui.dialogs.pending_unsaved_action.as_ref().unwrap().clone();
        let mut open = true;
        let mut resolved = false;
        let label: String = if self.document.savefile_path.is_empty() {
            "You have unsaved changes. What would you like to do?".to_string()
        } else {
            format!(
                "\"{}\" has unsaved changes. What would you like to do?",
                std::path::Path
                    ::new(&self.document.savefile_path)
                    .file_name()
                    .map(|s| s.to_string_lossy())
                    .unwrap_or_default()
            )
        };
        egui::Window
            ::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.label(&label);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        if self.document.savefile_path.is_empty() {
                            self.ui.dialogs.pending_after_save = Some(action.clone());
                            self.file_io.queue_file_action(PendingFileAction::Save);
                        } else {
                            self.file_io.save_to_current_path(&mut self.document);
                            self.ui.dialogs.pending_after_save = Some(action.clone());
                        }
                        resolved = true;
                    }
                    if !resolved && ui.button("Don't Save").clicked() {
                        self.execute_unsaved_action(action.clone());
                        resolved = true;
                    }
                    if !resolved && ui.button("Cancel").clicked() {
                        resolved = true;
                    }
                });
            });
        if !open || resolved {
            self.ui.dialogs.pending_unsaved_action = None;
        }
    }

    /// If the document has unsaved changes, store the action for later
    /// resolution; otherwise execute it immediately.
    pub(crate) fn guard_unsaved(&mut self, action: UnsavedWarningAction) {
        if self.document.dirty_since_last_autosave {
            self.ui.dialogs.pending_unsaved_action = Some(action);
        } else {
            self.execute_unsaved_action(action);
        }
    }

    /// Execute a deferred destructive action after the user has resolved the
    /// unsaved-changes warning (or after a save completes).
    fn execute_unsaved_action(&mut self, action: UnsavedWarningAction) {
        match action {
            UnsavedWarningAction::Quit => {
                self.ui.should_close = true;
            }
            UnsavedWarningAction::NewCanvas => {
                self.ui.dialogs.show_new_canvas_dialog = true;
            }
            UnsavedWarningAction::Load => {
                self.file_io.queue_file_action(PendingFileAction::Load);
            }
            UnsavedWarningAction::Import => {
                self.file_io.queue_file_action(PendingFileAction::Import);
            }
            UnsavedWarningAction::LoadPath(path) => {
                self.file_io.queue_load_direct(path);
            }
        }
    }

    /// Show the stamp-naming dialog when a new stamp has been loaded.
    fn show_stamp_naming_dialog(&mut self, ui: &mut egui::Ui) {
        let Some(mut pending) = self.ui.dialogs.pending_stamp_name.take() else {
            return;
        };
        let mut open = true;
        let mut cancelled = false;
        let label = format!("Size: {}×{}", pending.width, pending.height);
        egui::Window
            ::new("Name Your Stamp")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut pending.name).id_source("stamp_name_text")
                    );
                });
                ui.label(&label);
                if ui.button("Cancel").clicked() {
                    cancelled = true;
                }
                if ui.button("Add Stamp").clicked() && !pending.name.is_empty() {
                    let stamp_name = pending.name.clone();
                    let stamp_pixels = std::mem::take(&mut pending.pixels);
                    let stamp_w = pending.width;
                    let stamp_h = pending.height;
                    crate::stamp_library::add_stamp(
                        &mut self.stamp_library,
                        stamp_name.clone(),
                        stamp_pixels,
                        stamp_w,
                        stamp_h,
                        ui.ctx()
                    );
                    self.ui.toasts.message = Some((
                        format!("Stamp \"{stamp_name}\" added"),
                        Instant::now(),
                    ));
                }
            });
        if open && !cancelled {
            self.ui.dialogs.pending_stamp_name = Some(pending);
        }
    }

    /// Show the brush-import naming dialog when brushes have been loaded.
    fn show_brush_naming_dialog(&mut self, ui: &mut egui::Ui) {
        let Some(brushes) = &mut self.ui.dialogs.pending_brushes else {
            return;
        };
        let mut open = true;
        let mut confirmed = false;
        let mut cancelled = false;
        egui::Window
            ::new("Name Your Brushes")
            .collapsible(false)
            .resizable(true)
            .default_size([400.0, 300.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.label(format!("{} brush(es) imported — edit names below:", brushes.len()));
                ui.separator();

                let mut names_to_remove: Vec<usize> = Vec::new();
                egui::ScrollArea
                    ::vertical()
                    .max_height(ui.available_height() - 40.0)
                    .show(ui, |ui| {
                        for (i, brush) in brushes.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit
                                        ::singleline(&mut brush.name)
                                        .desired_width(150.0)
                                        .id_source(format!("brush_name_{i}"))
                                );
                                ui.label(format!("{}×{}", brush.width, brush.height));
                                if ui.button("Remove").clicked() {
                                    names_to_remove.push(i);
                                }
                            });
                        }
                    });

                names_to_remove.sort_unstable_by(|a, b| b.cmp(a));
                for i in names_to_remove {
                    brushes.remove(i);
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        cancelled = true;
                    }
                    if ui.button("Import All").clicked() && !brushes.is_empty() {
                        confirmed = true;
                    }
                });
            });
        if cancelled {
            self.ui.dialogs.pending_brushes = None;
        } else if confirmed {
            let all_brushes = self.ui.dialogs.pending_brushes.take().unwrap();
            let count = all_brushes.len();
            for brush in all_brushes {
                if !brush.name.is_empty() {
                    crate::brush_library::add_brush(
                        &mut self.brush_library,
                        brush.name,
                        brush.pixels,
                        brush.width,
                        brush.height,
                        brush.spacing,
                        ui.ctx()
                    );
                }
            }
            self.ui.toasts.message = Some((format!("Imported {count} brush(es)"), Instant::now()));
        } else if !open {
            self.ui.dialogs.pending_brushes = None;
        }
    }

    /// Show a brief toast notification (auto-dismissed after 2 seconds).
    fn show_toast(&mut self, ui: &mut egui::Ui) {
        let Some((message, triggered_at)) = &self.ui.toasts.message.clone() else {
            return;
        };
        if triggered_at.elapsed() < std::time::Duration::from_secs(2) {
            egui::Area
                ::new(egui::Id::new("stamp_toast"))
                .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText
                            ::new(message)
                            .color(egui::Color32::WHITE)
                            .background_color(egui::Color32::from_black_alpha(180))
                    );
                });
        } else {
            self.ui.toasts.message = None;
        }
    }

    /// Show a progress indicator in the bottom-right corner when an async
    /// operation is in-flight.
    fn show_progress_indicator(&mut self, ui: &mut egui::Ui) {
        let label = match self.ui.progress {
            ProgressState::Idle => {
                return;
            }
            ProgressState::Exporting => "Exporting…",
            ProgressState::Loading => "Loading…",
            ProgressState::Importing => "Importing…",
        };
        egui::Area
            ::new(egui::Id::new("progress_indicator"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    ui.label(label);
                });
            });
    }

    /// Add a file path to the recent-files list (dedup, max 10, most recent first).
    fn push_recent_file(&mut self, path: PathBuf) {
        if path.as_os_str().is_empty() {
            return;
        }
        self.ui.recent_files.retain(|p| p != &path);
        self.ui.recent_files.insert(0, path);
        self.ui.recent_files.truncate(10);
    }

    /// Path to the user-config JSON file (tool settings, preferences).
    fn config_path(&self) -> PathBuf {
        self.file_io.app_local_data_directory.join("config.json")
    }

    /// Persist current tool configuration and recent files to disk.
    fn save_config(&self) {
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
    fn handle_config_save(&mut self) {
        if
            self.ui.time_elapsed.saturating_sub(self.ui.last_autosave_time) >=
            Duration::from_mins(AUTOSAVE_INTERVAL_MINUTES)
        {
            self.save_config();
        }
    }


}

impl eframe::App for MyApp {
    /// Called every frame by eframe.
    ///
    /// Polls file dialog and save results, renders the canvas texture,
    /// draws top/left/right/center panels, shows error windows,
    /// and triggers autosave on a 2-minute interval.
    ///
    /// When the viewport is unfocused, sleeps to reduce CPU usage.
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        debug_snapshot(&self);
        self.poll_file_results(ui.ctx());

        if self.update_render_state(ui) {
            return;
        }

        self.sync_gpu_texture(frame, ui);

        let is_quitting = self.show_panels(ui);

        self.show_error_window(ui);
        self.show_delete_layer_dialog(ui);
        self.show_large_canvas_warning(ui);
        self.show_new_canvas_dialog(ui);
        self.show_unsaved_changes_warning(ui);
        self.show_stamp_naming_dialog(ui);
        self.show_brush_naming_dialog(ui);
        self.show_toast(ui);
        self.show_progress_indicator(ui);

        // Update window title to reflect unsaved changes.
        let filename = if self.document.savefile_path.is_empty() {
            "Untitled"
        } else {
            std::path::Path
                ::new(&self.document.savefile_path)
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("Untitled")
        };
        let new_title = if self.document.dirty_since_last_autosave {
            format!("{APP_NAME} — {filename} (unsaved)")
        } else {
            APP_NAME.to_string()
        };
        if self.ui.current_title != new_title {
            self.ui.current_title.clone_from(&new_title);
            ui.send_viewport_cmd(egui::ViewportCommand::Title(new_title));
        }

        if self.ui.should_close {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if is_quitting {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        self.handle_autosave();
        self.handle_config_save();
    }
}
