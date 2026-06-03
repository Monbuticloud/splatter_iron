//! Top-level application state, identity constants, export formats, autosave
//! loop, and wiring between the document, tools, undo history, and file IO.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

#[cfg(feature = "debug-snapshot")]
use crate::debug::debug_snapshot;
use directories::ProjectDirs;
use eframe::egui::{self};
use eframe::egui_wgpu::wgpu;
use serde::Deserialize;
use serde::Serialize;

use crate::asset_library::Library;
use crate::brush_library::BrushEntry;
use crate::canvas::Canvas;
use crate::canvas::CurrentTool;
use crate::canvas::RenderState;
use crate::document::Document;
use crate::file_io::DialogManager;
use crate::file_io::ExportManager;
use crate::file_io::LoadImportManager;
use crate::file_io::SaveManager;
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

/// File extension for archive files (xz-compressed JSON).
pub const ARCHIVE_EXTENSION: &str = ".splatterarchive";
/// File-dialog filter name for `.splatterarchive` files.
pub const ARCHIVE_FILTER_NAME: &str = "SplatterArchive";
/// Default export name for archive files.
pub const DEFAULT_ARCHIVE_NAME: &str = "canvas.splatterarchive";

/// Preset canvas sizes shown in the "New Canvas" dialog.
pub(crate) const NEW_CANVAS_PRESETS: &[(&str, u32, u32)] = &[
    ("XS", 800, 600),
    ("S", 1280, 960),
    ("M", 2000, 1500),
    ("L", 2560, 1920),
    ("XL", 3200, 2400),
];

// --- Performance constants ---
/// Sleep duration (ms) while the window is unfocused, used by
/// `RenderState::UnfocusedFrozen` to minimise repaint overhead.
pub(crate) const UNFOCUSED_SLEEP_MILLISECONDS: u64 = 50;

// --- Memory warning threshold ---
/// Threshold (in bytes) above which creating a new canvas shows a
/// confirmation dialog. 500 MB is a safe boundary — output_rgba + one layer
/// + blend buffer at 8000×8000 is ~768 MB.
pub(crate) const MEMORY_WARNING_THRESHOLD: u64 = 500_000_000;

/// Estimate the minimum memory footprint (bytes) for a canvas of the given
/// dimensions and layer count: output_rgba (w×h×4) + each layer's pixel data
/// (w×h×4 per layer) + blend buffer overhead (w×h×4).
/// Pass `layer_count = 1` for a new canvas (one default layer).
pub(crate) fn estimate_canvas_memory(width: u32, height: u32, layer_count: usize) -> u64 {
    let pixels = u64::from(width) * u64::from(height);
    // output_rgba + blend_buffer + each layer's pixel data
    pixels * 4 * (2 + layer_count as u64)
}
/// Repaint-interval multiplier applied when the render state is
/// `IdleThrottled` — the normal repaint delay is multiplied by this
/// value to reduce CPU/GPU load during idle periods.
pub(crate) const REPAINT_DELAY_MULTIPLIER: u32 = 5;

// --- Autosave interval ---
/// Interval (minutes) between automatic saves of the current canvas.
/// The autosave loop in `MyApp` checks this duration against the last
/// autosave timestamp to decide when to trigger a background save.
pub(crate) const AUTOSAVE_INTERVAL_MINUTES: u64 = 2;

// --- Image import extensions ---
/// Serialization wrapper for tool config + recent files.
#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedConfig {
    /// Tool settings (current tool, color, radius, etc.).
    pub tool_configuration: ToolConfiguration,
    /// Recently opened/saved file paths.
    pub recent_files: Vec<PathBuf>,
}

/// File-extension list accepted by the image-import dialog (19 formats).
pub const IMPORT_EXTENSIONS: &[&str] = &[
    "avif", "png", "jpg", "jpeg", "webp", "gif", "tiff", "tif", "tga", "ico", "pnm", "pgm", "ppm",
    "pbm", "pam", "qoi", "exr", "hdr", "ff",
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
    /// All dialogs closed, default canvas dimensions (2000×1500),
    /// no pending stamps, brushes, or unsaved-warning actions.
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
    /// Empty error list — no active errors.
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
    /// No active toast notification.
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
    /// Saving the canvas to disk (either autosave or manual save).
    Saving,
    /// Periodic autosave in progress.
    Autosaving,
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
    /// Virtual cursor position in float-pixel space for brush stabilization.
    /// `None` when stabilization is disabled or no cursor has been placed yet.
    pub stabilized_cursor: Option<(f32, f32)>,
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
    /// Brief completion message shown in the canvas status bar after a save
    /// or autosave finishes (e.g. "Saved", "Autosaved"). Erased after ~2
    /// seconds via the status bar drawing logic.
    pub last_status_message: Option<(&'static str, Instant)>,
    /// Set to `true` when the app should close (e.g., after unsaved-changes
    /// resolution chooses Quit).
    pub should_close: bool,
    /// Pan offset from center, in screen pixels.
    pub pan_offset: egui::Vec2,
    /// Current zoom level (1.0 = fit to screen).
    pub zoom: f32,
    /// Cached grid overlay shapes, recomputed when any cache-key dimension changes.
    pub grid_cache: Option<(Vec<egui::Shape>, u32, u32, u32)>,
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
            stabilized_cursor: None,
            undo_redo_steps_multiplier: 1,
            recent_files: Vec::new(),
            last_export_format: 1,
            current_title: APP_NAME.to_string(),
            dialogs: DialogState::default(),
            errors: ErrorState::default(),
            toasts: ToastState::default(),
            progress: ProgressState::Idle,
            last_status_message: None,
            should_close: false,
            pan_offset: egui::Vec2::ZERO,
            zoom: 1.0,
            grid_cache: None,
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
    /// File dialog state machine.
    pub dialog_manager: DialogManager,
    /// Async save orchestration.
    pub save_manager: SaveManager,
    /// Async export orchestration.
    pub export_manager: ExportManager,
    /// Async load/import orchestration.
    pub load_import_manager: LoadImportManager,
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

        let project_dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
            .expect("Couldn't resolve app dir");
        let data_dir = project_dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&data_dir).expect("Couldn't create data dir");
        std::fs::create_dir_all(&data_dir.join("autosaves")).expect("Couldn't create autosave dir");

        // Query the device's max 2D texture dimension for the new-canvas slider.
        // Falls back to 8192 (WebGPU minimum) when the wgpu backend is unavailable.
        let max_texture_dimension = creation_context
            .wgpu_render_state
            .as_ref()
            .map(|rs| rs.device.limits().max_texture_dimension_2d)
            .unwrap_or(8192);

        let stamp_library: Library<StampEntry> = Library::load_from_disk(&data_dir);
        let brush_library: Library<BrushEntry> = Library::load_from_disk(&data_dir);

        let gpu_texture = creation_context
            .wgpu_render_state
            .as_ref()
            .map(|render_state| {
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
                    }),
                );
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let mut renderer = render_state.renderer.write();
                let texture_id = renderer.register_native_texture(
                    &render_state.device,
                    &view,
                    wgpu::FilterMode::Linear,
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
                std::fs::File::open(data_dir.join("config.json"))
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
            dialog_manager: DialogManager::new(dialog_sender, dialog_receiver),
            save_manager: SaveManager::new(
                save_result_sender,
                save_result_receiver,
                data_dir.clone(),
            ),
            export_manager: ExportManager::new(std::sync::Arc::new(
                crate::files::DefaultExportStrategy,
            )),
            load_import_manager: LoadImportManager::new(),
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

impl eframe::App for MyApp {
    /// Called every frame by eframe.
    ///
    /// Polls file dialog and save results, renders the canvas texture,
    /// draws top/left/right/center panels, shows error windows,
    /// and triggers autosave on a 2-minute interval.
    ///
    /// When the viewport is unfocused, sleeps to reduce CPU usage.
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        #[cfg(feature = "debug-snapshot")]
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

        // Update window title to reflect unsaved changes.
        let filename = if self.document.savefile_path.is_empty() {
            "Untitled"
        } else {
            std::path::Path::new(&self.document.savefile_path)
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
