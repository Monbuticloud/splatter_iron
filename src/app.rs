//! Top-level application state, identity constants, export formats, autosave
//! loop, and wiring between the document, tools, undo history, and file IO.

use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{ self, Panel };
use eframe::egui_wgpu::wgpu;
use directories::ProjectDirs;

use crate::canvas::{ Canvas, RenderState };
use crate::document::Document;
use crate::file_io::{ FileIO, SaveKind };
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

/// Maximum canvas dimension (8192 = 2¹³). This is the de-facto max 2D texture
/// size guaranteed across GPU backends in wgpu/WebGPU, and avoids
/// platform-specific allocation issues on older hardware. Going beyond this
/// would risk `OUT_OF_MEMORY` on integrated GPUs and driver crashes on DX11
/// / OpenGL ES 3.0 devices that cap at 8192.
const MAX_CANVAS_DIMENSION: u32 = 8192;

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
const REPAINT_DELAY_MULTIPLIER: u32 = 5;

// --- Autosave interval ---
const AUTOSAVE_INTERVAL_MINUTES: u64 = 2;

// --- Image import extensions ---
/// File-extension list accepted by the image-import dialog (19 formats).
pub const IMPORT_EXTENSIONS: &[&str] = &[
    "avif", "png", "jpg", "jpeg", "webp", "gif", "tiff", "tif",
    "tga", "ico", "pnm", "pgm", "ppm", "pbm", "pam", "qoi", "exr", "hdr", "ff",
];

/// File extension list and image format for an export target.
pub struct ExportInformation {
    pub extensions: &'static [&'static str],
    pub fmt: image::ImageFormat,
}

/// Lookup table for all supported export formats.
pub const EXPORT_FORMATS: &[(&str, ExportInformation)] = &[
    ("AVIF", ExportInformation { extensions: &["avif"], fmt: image::ImageFormat::Avif }),
    ("PNG", ExportInformation { extensions: &["png"], fmt: image::ImageFormat::Png }),
    ("JPEG", ExportInformation { extensions: &["jpg", "jpeg"], fmt: image::ImageFormat::Jpeg }),
    ("WebP", ExportInformation { extensions: &["webp"], fmt: image::ImageFormat::WebP }),
    ("GIF", ExportInformation { extensions: &["gif"], fmt: image::ImageFormat::Gif }),
    ("TIFF", ExportInformation { extensions: &["tiff", "tif"], fmt: image::ImageFormat::Tiff }),
    ("TGA", ExportInformation { extensions: &["tga"], fmt: image::ImageFormat::Tga }),
    ("ICO", ExportInformation { extensions: &["ico"], fmt: image::ImageFormat::Ico }),
    (
        "PNM",
        ExportInformation {
            extensions: &["pnm", "pgm", "ppm", "pbm", "pam"],
            fmt: image::ImageFormat::Pnm,
        },
    ),
    ("QOI", ExportInformation { extensions: &["qoi"], fmt: image::ImageFormat::Qoi }),
    ("EXR", ExportInformation { extensions: &["exr"], fmt: image::ImageFormat::OpenExr }),
    ("HDR", ExportInformation { extensions: &["hdr"], fmt: image::ImageFormat::Hdr }),
    ("Farbfeld", ExportInformation { extensions: &["ff"], fmt: image::ImageFormat::Farbfeld }),
];

/// A stamp image awaiting a user-provided name before being added to the library.
pub struct PendingStamp {
    pub pixels: Vec<egui::Color32>,
    pub width: u32,
    pub height: u32,
    pub name: String,
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
    /// Error messages displayed in the error overlay.
    pub displayed_error_list: Vec<String>,
    /// Layer index pending deletion confirmation, if any.
    pub pending_layer_for_deletion: Option<usize>,
    /// Whether the "New Canvas" dialog is currently open.
    pub show_new_canvas_dialog: bool,
    /// Width input for the new canvas dialog (in pixels).
    pub new_canvas_width: u32,
    /// Height input for the new canvas dialog (in pixels).
    pub new_canvas_height: u32,
    /// A stamp image awaiting a name from the user before being added to the library.
    pub pending_stamp_name: Option<PendingStamp>,
    /// Transient toast message and the instant it was triggered.
    pub toast_message: Option<(String, Instant)>,
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
            displayed_error_list: Vec::new(),
            pending_layer_for_deletion: None,
            show_new_canvas_dialog: false,
            new_canvas_width: 2000,
            new_canvas_height: 1500,
            pending_stamp_name: None,
            toast_message: None,
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
    pub stamp_library: crate::stamp_library::StampLibrary,
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

        let stamp_library = crate::stamp_library::StampLibrary::load_from_disk(&data_dir);

        let gpu_texture = creation_context.wgpu_render_state.as_ref().map(|render_state| {
            let width = canvas.width;
            let height = canvas.height;
            let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
            let texture = render_state.device.create_texture(&wgpu::TextureDescriptor {
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
            ),
            ui: UIState::default(),
            gpu_texture,
            stamp_library,
        }
    }

    /// Recreate the wgpu GPU texture after a canvas resize.
    ///
    /// Uses `update_egui_texture_from_wgpu_texture` to keep the same
    /// `egui::TextureId`, avoiding stale entries in the renderer's map.
    ///
    /// # Panics
    ///
    /// Panics if `render_state.renderer.write()` is poisoned (lock contention)
    /// or if the wgpu device has been lost.
    pub fn recreate_gpu_texture(&mut self, frame: &mut eframe::Frame) {
        let Some(render_state) = frame.wgpu_render_state() else { return };
        let Some(gpu) = &mut self.gpu_texture else { return };
        let width = self.document.canvas.width;
        let height = self.document.canvas.height;
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        gpu.texture = render_state.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("splatter_iron_canvas"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = gpu.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut renderer = render_state.renderer.write();
        renderer.update_egui_texture_from_wgpu_texture(
            &render_state.device,
            &view,
            wgpu::FilterMode::Linear,
            gpu.texture_id,
        );
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
        // Poll dialog results and save results before anything else.
        self.file_io.poll_dialog_results(&mut self.document, &mut self.undo, &mut self.ui.displayed_error_list);
        self.file_io.poll_save_results(&mut self.document, &mut self.ui.displayed_error_list);

        // Transfer a newly loaded stamp image into the stamp library.
        if let Some((pixels, w, h, name)) = self.file_io.loaded_stamp_data.take() {
            self.ui.pending_stamp_name = Some(crate::app::PendingStamp {
                pixels,
                width: w,
                height: h,
                name,
            });
        }

        // Create egui textures for stamps (needs ctx — available once per frame).
        self.stamp_library.create_textures(ui.ctx());

        if !ui.ctx().input(|i| i.viewport().focused.unwrap_or(true)) {
            std::thread::sleep(std::time::Duration::from_millis(UNFOCUSED_SLEEP_MILLISECONDS));
            self.ui.render_state = RenderState::UnfocusedFrozen;
            return;
        }
        let predicted_delta_time = Duration::from_secs_f32(
            ui.ctx().input(|i| i.predicted_dt).max(0.0)
        );
        let real_delta_time = Duration::from_secs_f32(
            ui.ctx().input(|i| i.stable_dt).max(0.0)
        );

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
                return;
            }
        }

        // Recreate GPU texture if canvas dimensions have changed
        if let Some(gpu) = &self.gpu_texture {
            let texture_size = gpu.texture.size();
            if texture_size.width != self.document.canvas.width || texture_size.height != self.document.canvas.height {
                self.recreate_gpu_texture(frame);
            }
        }

        // Render layers to texture if needed
        if self.gpu_texture.is_some() {
            if self.document.canvas.render_next_frame {
                let dirty = self.document.blend_to_output();
                if let Some(ref gpu) = self.gpu_texture {
                    self.document.upload_to_gpu(&gpu.queue, &gpu.texture, &dirty);
                }
            }
        } else if self.document.canvas.render_next_frame || self.document.canvas.rendered_layers.is_none() {
            self.document.render_to_texture(ui);
        }

        let is_quitting = Panel::top("top").show_inside(ui, |ui| self.show_top_panel(ui)).inner;

        Panel::left("side").show_inside(ui, |ui| self.show_left_panel(ui));

        Panel::right("right").show_inside(ui, |ui| self.show_right_panel(ui));

        egui::CentralPanel::default().show_inside(ui, |ui| self.show_central_panel(ui));

        if !self.ui.displayed_error_list.is_empty() {
            let mut open = true;
            let mut to_dismiss: Vec<usize> = Vec::new();
            egui::Window
                ::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ui, |ui| {
                    for (index, msg) in self.ui.displayed_error_list.iter().enumerate() {
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
                            to_dismiss.extend(0..self.ui.displayed_error_list.len());
                        }
                    });
                });

            // Remove in descending order so earlier removals don't shift later indices.
            to_dismiss.sort_unstable_by(|a, b| b.cmp(a));
            to_dismiss.dedup();
            for i in to_dismiss {
                self.ui.displayed_error_list.remove(i);
            }
            if !open {
                self.ui.displayed_error_list.clear();
            }
        }

        // --- New Canvas dialog ---
        if self.ui.show_new_canvas_dialog {
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
                                self.ui.new_canvas_width = width;
                                self.ui.new_canvas_height = height;
                            }
                        }
                    });
                    ui.separator();
                    ui.label("Custom:");
                    ui.add(
                        egui::Slider::new(&mut self.ui.new_canvas_width, 4..=MAX_CANVAS_DIMENSION)
                            .text("Width"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.ui.new_canvas_height, 4..=MAX_CANVAS_DIMENSION)
                            .text("Height"),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            let canvas = Canvas::new(self.ui.new_canvas_width, self.ui.new_canvas_height);
                            self.document.replace_canvas(canvas, &mut self.undo);
                            self.tool_configuration.previous_tool = None;
                            self.tool_configuration.previous_cursor_position = None;
                            self.ui.show_new_canvas_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.ui.show_new_canvas_dialog = false;
                        }
                    });
                });
            if !open {
                self.ui.show_new_canvas_dialog = false;
            }
        }

        // --- Stamp naming dialog ---
        if self.ui.pending_stamp_name.is_some() {
            // Take ownership of the pending stamp to avoid borrow conflicts.
            let mut pending = self.ui.pending_stamp_name.take().unwrap();
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
                        ui.text_edit_singleline(&mut pending.name);
                    });
                    ui.label(&label);
                    if ui.button("Add Stamp").clicked() && !pending.name.is_empty() {
                        let stamp_name = pending.name.clone();
                        let stamp_pixels = std::mem::take(&mut pending.pixels);
                        let stamp_w = pending.width;
                        let stamp_h = pending.height;
                        self.stamp_library.add(
                            stamp_name.clone(),
                            stamp_pixels,
                            stamp_w,
                            stamp_h,
                            ui.ctx(),
                        );
                        self.ui.toast_message = Some((
                            format!("Stamp \"{stamp_name}\" added"),
                            Instant::now(),
                        ));
                    }
                });
            if open {
                self.ui.pending_stamp_name = Some(pending);
            }
        }

        // --- Toast notification ---
        if let Some((message, triggered_at)) = &self.ui.toast_message.clone() {
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
                self.ui.toast_message = None;
            }
        }

        if is_quitting {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Autosave every AUTOSAVE_INTERVAL_MINUTES minutes, but only if the canvas has been modified.
        if
            self.document.dirty_since_last_autosave &&
            self.ui.time_elapsed.saturating_sub(self.ui.last_autosave_time) >= Duration::from_mins(AUTOSAVE_INTERVAL_MINUTES)
        {
            self.ui.last_autosave_time = self.ui.time_elapsed;
            self.ui.times_autosaved += 1;
            self.file_io.trigger_async_save(&self.document, SaveKind::Autosave);
        }
    }
}
