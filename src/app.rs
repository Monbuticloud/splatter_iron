//! Top-level application state, identity constants, export formats, autosave
//! loop, and wiring between the document, tools, undo history, and file IO.

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use directories::ProjectDirs;
use eframe::egui::Panel;
use eframe::egui::{self};
use eframe::egui_wgpu::wgpu;

use crate::asset_library::Library;
use crate::brush_library::BrushEntry;
use crate::canvas::Canvas;
use crate::canvas::CurrentTool;
use crate::canvas::RenderState;
use crate::document::Document;
use crate::file_io::FileIO;
use crate::file_io::SaveKind;
use crate::stamp_library::StampEntry;
use crate::tool_configuration::ToolConfiguration;
use crate::undo_history::UndoHistory;

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
const NEW_CANVAS_PRESETS: &[(&str, u32, u32)] = &[
    ("XS", 800, 600),
    ("S", 1280, 960),
    ("M", 2000, 1500),
    ("L", 2560, 1920),
    ("XL", 3200, 2400),
];

// --- Performance constants ---
const UNFOCUSED_SLEEP_MILLISECONDS: u64 = 50;

// --- Memory warning threshold ---
/// Threshold (in bytes) above which creating a new canvas shows a
/// confirmation dialog. 500 MB is a safe boundary — output_rgba + one layer
/// + blend buffer at 8000×8000 is ~768 MB.
const MEMORY_WARNING_THRESHOLD: u64 = 500_000_000;

/// Estimate the minimum memory footprint (bytes) for a canvas of the given
/// dimensions: output_rgba (w×h×4) + one layer (w×h×4) + blend buffer
/// overhead (w×h×4). The actual footprint is higher with multiple layers.
fn estimate_canvas_memory(width: u32, height: u32) -> u64 {
    let pixels = u64::from(width) * u64::from(height);
    pixels * 12
}
const REPAINT_DELAY_MULTIPLIER: u32 = 5;

// --- Autosave interval ---
const AUTOSAVE_INTERVAL_MINUTES: u64 = 2;

// --- Image import extensions ---
/// File-extension list accepted by the image-import dialog (19 formats).
pub const IMPORT_EXTENSIONS: &[&str] = &[
    "avif", "png", "jpg", "jpeg", "webp", "gif", "tiff", "tif", "tga", "ico", "pnm", "pgm", "ppm",
    "pbm", "pam", "qoi", "exr", "hdr", "ff",
];

/// File extension list and image format for an export target.
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

/// Dialog-related state: open/closed flags, input values, pending confirmations.
pub struct DialogState {
    /// Layer index pending deletion confirmation, if any.
    pub pending_layer_for_deletion: Option<usize>,
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
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            pending_layer_for_deletion: None,
            show_new_canvas_dialog: false,
            new_canvas_width: 2000,
            new_canvas_height: 1500,
            pending_large_canvas: None,
            pending_stamp_name: None,
            pending_brushes: None,
        }
    }
}

/// Error messages displayed in the error overlay.
pub struct ErrorState {
    pub list: Vec<String>,
}

impl Default for ErrorState {
    fn default() -> Self {
        Self { list: Vec::new() }
    }
}

/// Transient toast notification.
pub struct ToastState {
    /// The message text and the instant it was triggered.
    pub message: Option<(String, Instant)>,
}

impl Default for ToastState {
    fn default() -> Self {
        Self { message: None }
    }
}

/// UI-level state that doesn't belong to any domain module.
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
    /// Cached window title — updated when dirty flag changes.
    pub current_title: String,
    /// Dialog-related state.
    pub dialogs: DialogState,
    /// Error messages displayed in the error overlay.
    pub errors: ErrorState,
    /// Transient toast notification.
    pub toasts: ToastState,
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
            current_title: crate::app::APP_NAME.to_string(),
            dialogs: DialogState::default(),
            errors: ErrorState::default(),
            toasts: ToastState::default(),
        }
    }
}

/// WGPU GPU texture and associated state for partial canvas uploads.
///
/// Created once during `MyApp` construction and updated on canvas resize.
/// The `queue` is used each frame to upload only the dirty sub-region
/// via `wgpu::Queue::write_texture`.
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
                let texture = render_state
                    .device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("splatter_iron_canvas"),
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });
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

        Self {
            document: Document::new(canvas),
            tool_configuration: ToolConfiguration::default(),
            undo: UndoHistory::new(pixel_count),
            file_io: FileIO::new(
                dialog_sender,
                dialog_receiver,
                save_result_sender,
                save_result_receiver,
                data_dir,
                Box::new(crate::files::DefaultExportStrategy),
            ),
            ui: UIState {
                max_texture_dimension,
                ..UIState::default()
            },
            gpu_texture,
            stamp_library,
            brush_library,
        }
    }

    /// Recreate the wgpu GPU texture after a canvas resize.
    ///
    /// Uses `update_egui_texture_from_wgpu_texture` to keep the same
    /// `egui::TextureId`, avoiding stale entries in the renderer's map.
    ///
    /// If the canvas dimensions exceed the device's `max_texture_dimension_2d`,
    /// an error is pushed to `displayed_error_list` and the texture is not
    /// recreated (the old texture remains, now stale).
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the renderer lock cannot be acquired within
    /// 10 seconds (parking_lot deadlock detection). Panics if the wgpu device
    /// has been lost.
    pub fn recreate_gpu_texture(&mut self, frame: &mut eframe::Frame) {
        let Some(render_state) = frame.wgpu_render_state() else {
            return;
        };
        let Some(gpu) = &mut self.gpu_texture else {
            return;
        };
        let width = self.document.canvas.width;
        let height = self.document.canvas.height;
        let max_dim = render_state.device.limits().max_texture_dimension_2d;
        if width > max_dim || height > max_dim {
            self.ui.errors.list.push(format!(
                "Canvas too large for GPU: {width}×{height} exceeds device max \
                 texture dimension of {max_dim}. The display may be incomplete."
            ));
            return;
        }
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        gpu.texture = render_state
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("splatter_iron_canvas"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
        let view = gpu
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut renderer = render_state.renderer.write();
        renderer.update_egui_texture_from_wgpu_texture(
            &render_state.device,
            &view,
            wgpu::FilterMode::Linear,
            gpu.texture_id,
        );
    }
}

// --- Frame-helper methods (called once per frame from `ui()`) ---

impl MyApp {
    /// Poll file-dialog and save-result channels and transfer loaded
    /// stamp/brush data into pending-dialog state.
    fn poll_file_results(&mut self, ctx: &egui::Context) {
        self.file_io.poll_dialog_results(
            &mut self.document,
            &mut self.undo,
            &mut self.ui.errors.list,
        );
        self.file_io
            .poll_save_results(&mut self.document, &mut self.ui.errors.list);

        if let Some((pixels, w, h, name)) = self.file_io.loaded_stamp_data.take() {
            self.ui.dialogs.pending_stamp_name = Some(crate::app::PendingStamp {
                pixels,
                width: w,
                height: h,
                name,
                spacing: 25,
            });
        }

        if let Some(tips) = self.file_io.loaded_brush_data.take() {
            let pending: Vec<PendingStamp> = tips
                .into_iter()
                .map(|tip| PendingStamp {
                    pixels: tip.pixels,
                    width: tip.width,
                    height: tip.height,
                    name: tip.name,
                    spacing: tip.spacing,
                })
                .collect();
            self.ui.dialogs.pending_brushes = Some(pending);
        }

        self.stamp_library.create_textures(ctx);
        self.brush_library.create_textures(ctx);
    }

    /// Advance the render-state machine and return `true` if the frame should
    /// be skipped (viewport unfocused or frozen).
    fn update_render_state(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.ctx().input(|i| i.viewport().focused.unwrap_or(true)) {
            std::thread::sleep(std::time::Duration::from_millis(
                UNFOCUSED_SLEEP_MILLISECONDS,
            ));
            self.ui.render_state = RenderState::UnfocusedFrozen;
            return true;
        }
        let predicted_delta_time =
            Duration::from_secs_f32(ui.ctx().input(|i| i.predicted_dt).max(0.0));
        let real_delta_time = Duration::from_secs_f32(ui.ctx().input(|i| i.stable_dt).max(0.0));

        self.ui.time_elapsed += real_delta_time;

        match self.ui.render_state {
            RenderState::ActiveWake(duration) => {
                let remaining = duration.saturating_sub(predicted_delta_time);
                if remaining.is_zero() {
                    self.ui.render_state = RenderState::IdleThrottled;
                    ui.request_repaint_after(predicted_delta_time * REPAINT_DELAY_MULTIPLIER);
                } else {
                    self.ui.render_state = RenderState::ActiveWake(remaining);
                }
            }
            RenderState::IdleThrottled => {
                ui.request_repaint_after(predicted_delta_time * REPAINT_DELAY_MULTIPLIER);
            }
            RenderState::UnfocusedFrozen => {
                self.ui.render_state = RenderState::IdleThrottled;
                return true;
            }
        }
        false
    }

    /// Recreate the GPU texture if dimensions changed, then blend and upload.
    fn sync_gpu_texture(&mut self, frame: &mut eframe::Frame, ui: &mut egui::Ui) {
        if let Some(gpu) = &self.gpu_texture {
            let texture_size = gpu.texture.size();
            if texture_size.width != self.document.canvas.width
                || texture_size.height != self.document.canvas.height
            {
                self.recreate_gpu_texture(frame);
            }
        }

        let needs_blend = self.document.canvas.dirty_rect.needs_reblend();

        if self.gpu_texture.is_some() {
            if needs_blend {
                let dirty = self.document.blend_to_output();
                if let Some(ref gpu) = self.gpu_texture {
                    self.document
                        .upload_to_gpu(&gpu.queue, &gpu.texture, &dirty);
                }
            }
        } else if needs_blend || self.document.canvas.rendered_layers.is_none() {
            self.document.render_to_texture(ui);
        }
    }

    /// Render all four panels (top, left, right, centre) and return whether
    /// the user requested to quit (via the top panel).
    fn show_panels(&mut self, ui: &mut egui::Ui) -> bool {
        let is_quitting = Panel::top("top")
            .show_inside(ui, |ui| self.show_top_panel(ui))
            .inner;

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
        egui::Window::new("Error")
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
        egui::Window::new("Large Canvas Warning")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.label(format!(
                    "This canvas ({w}×{h}) may use up to ~{estimated_mb} MB of RAM.\n\
                     Proceed? This cannot be undone."
                ));
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

    /// Show the "New Canvas" preset / custom-size dialog.
    fn show_new_canvas_dialog(&mut self, ui: &mut egui::Ui) {
        if !self.ui.dialogs.show_new_canvas_dialog {
            return;
        }
        let mut open = true;
        egui::Window::new("New Canvas")
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
                    egui::Slider::new(
                        &mut self.ui.dialogs.new_canvas_width,
                        4..=self.ui.max_texture_dimension,
                    )
                    .text("Width"),
                );
                ui.add(
                    egui::Slider::new(
                        &mut self.ui.dialogs.new_canvas_height,
                        4..=self.ui.max_texture_dimension,
                    )
                    .text("Height"),
                );
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        let mem = estimate_canvas_memory(
                            self.ui.dialogs.new_canvas_width,
                            self.ui.dialogs.new_canvas_height,
                        );
                        if mem > MEMORY_WARNING_THRESHOLD {
                            self.ui.dialogs.pending_large_canvas =
                                Some((self.ui.dialogs.new_canvas_width, self.ui.dialogs.new_canvas_height));
                        } else {
                            let canvas = Canvas::new(
                                self.ui.dialogs.new_canvas_width,
                                self.ui.dialogs.new_canvas_height,
                            );
                            self.document.replace_canvas(canvas, &mut self.undo);
                            self.ui.previous_tool = None;
                            self.ui.previous_cursor_position = None;
                            self.ui.dialogs.show_new_canvas_dialog = false;
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

    /// Show the stamp-naming dialog when a new stamp has been loaded.
    fn show_stamp_naming_dialog(&mut self, ui: &mut egui::Ui) {
        let Some(mut pending) = self.ui.dialogs.pending_stamp_name.take() else {
            return;
        };
        let mut open = true;
        let label = format!("Size: {}×{}", pending.width, pending.height);
        egui::Window::new("Name Your Stamp")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut pending.name)
                            .id_source("stamp_name_text"),
                    );
                });
                ui.label(&label);
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
                        ui.ctx(),
                    );
                    self.ui.toasts.message =
                        Some((format!("Stamp \"{stamp_name}\" added"), Instant::now()));
                }
            });
        if open {
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
        egui::Window::new("Name Your Brushes")
            .collapsible(false)
            .resizable(true)
            .default_size([400.0, 300.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.label(format!(
                    "{} brush(es) imported — edit names below:",
                    brushes.len()
                ));
                ui.separator();

                let mut names_to_remove: Vec<usize> = Vec::new();
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height() - 40.0)
                    .show(ui, |ui| {
                        for (i, brush) in brushes.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut brush.name)
                                        .desired_width(150.0)
                                        .id_source(format!("brush_name_{i}")),
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
                if ui.button("Import All").clicked() && !brushes.is_empty() {
                    confirmed = true;
                }
            });
        if confirmed {
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
                        ui.ctx(),
                    );
                }
            }
            self.ui.toasts.message =
                Some((format!("Imported {count} brush(es)"), Instant::now()));
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
            egui::Area::new(egui::Id::new("stamp_toast"))
                .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(message)
                            .color(egui::Color32::WHITE)
                            .background_color(egui::Color32::from_black_alpha(180)),
                    );
                });
        } else {
            self.ui.toasts.message = None;
        }
    }

    /// Trigger an autosave if the canvas is dirty and enough time has elapsed.
    fn handle_autosave(&mut self) {
        if self.document.dirty_since_last_autosave
            && self
                .ui
                .time_elapsed
                .saturating_sub(self.ui.last_autosave_time)
                >= Duration::from_mins(AUTOSAVE_INTERVAL_MINUTES)
        {
            self.ui.last_autosave_time = self.ui.time_elapsed;
            self.ui.times_autosaved += 1;
            self.file_io
                .trigger_async_save(&mut self.document, SaveKind::Autosave);
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
        self.poll_file_results(ui.ctx());

        if self.update_render_state(ui) {
            return;
        }

        self.sync_gpu_texture(frame, ui);

        let is_quitting = self.show_panels(ui);

        self.show_error_window(ui);
        self.show_large_canvas_warning(ui);
        self.show_new_canvas_dialog(ui);
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

        if is_quitting {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        self.handle_autosave();
    }
}
