use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{ self, Panel };
use eframe::egui_wgpu::wgpu;
use directories::ProjectDirs;

use crate::canvas::{ Canvas, RenderState };
use crate::document::Document;
use crate::file_io::{ FileIO, SaveKind };
use crate::tool_config::ToolConfig;
use crate::undo_history::UndoHistory;

// --- App identity constants ---
pub const APP_QUALIFIER: &str = "com";
pub const APP_ORG: &str = "Monbuticloud";
pub const APP_NAME: &str = "SplatterIron";

// --- Canvas & save file constants ---
pub const CANVAS_EXT: &str = ".splattercanvas";
pub const FILE_FILTER_NAME: &str = "SplatterCanvas";
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
const UNFOCUSED_SLEEP_MS: u64 = 50;
const REPAINT_DELAY_MULT: u32 = 5;

// --- Autosave interval ---
const AUTOSAVE_INTERVAL_MINS: u64 = 2;

// --- Image import extensions ---
pub const IMPORT_EXTENSIONS: &[&str] = &[
    "avif", "png", "jpg", "jpeg", "webp", "gif", "tiff", "tif",
    "tga", "ico", "pnm", "pgm", "ppm", "pbm", "pam", "qoi", "exr", "hdr", "ff",
];

pub struct ExportInfo {
    pub extensions: &'static [&'static str],
    pub fmt: image::ImageFormat,
}

/// Lookup table for all supported export formats.
pub const EXPORT_FORMATS: &[(&str, ExportInfo)] = &[
    ("AVIF", ExportInfo { extensions: &["avif"], fmt: image::ImageFormat::Avif }),
    ("PNG", ExportInfo { extensions: &["png"], fmt: image::ImageFormat::Png }),
    ("JPEG", ExportInfo { extensions: &["jpg", "jpeg"], fmt: image::ImageFormat::Jpeg }),
    ("WebP", ExportInfo { extensions: &["webp"], fmt: image::ImageFormat::WebP }),
    ("GIF", ExportInfo { extensions: &["gif"], fmt: image::ImageFormat::Gif }),
    ("TIFF", ExportInfo { extensions: &["tiff", "tif"], fmt: image::ImageFormat::Tiff }),
    ("TGA", ExportInfo { extensions: &["tga"], fmt: image::ImageFormat::Tga }),
    ("ICO", ExportInfo { extensions: &["ico"], fmt: image::ImageFormat::Ico }),
    (
        "PNM",
        ExportInfo {
            extensions: &["pnm", "pgm", "ppm", "pbm", "pam"],
            fmt: image::ImageFormat::Pnm,
        },
    ),
    ("QOI", ExportInfo { extensions: &["qoi"], fmt: image::ImageFormat::Qoi }),
    ("EXR", ExportInfo { extensions: &["exr"], fmt: image::ImageFormat::OpenExr }),
    ("HDR", ExportInfo { extensions: &["hdr"], fmt: image::ImageFormat::Hdr }),
    ("Farbfeld", ExportInfo { extensions: &["ff"], fmt: image::ImageFormat::Farbfeld }),
];

/// UI-level state that doesn't belong to any domain module.
pub struct UIState {
    pub render_state: RenderState,
    pub time_elapsed: Duration,
    pub times_autosaved: u32,
    pub last_autosave_time: Duration,
    pub displayed_error_list: Vec<String>,
    pub pending_layer_for_deletion: Option<usize>,
    pub show_new_canvas_dialog: bool,
    pub new_canvas_width: u32,
    pub new_canvas_height: u32,
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
        }
    }
}

/// WGPU GPU texture and associated state for partial canvas uploads.
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub texture_id: egui::TextureId,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

/// Top-level application state owned by eframe: document, tools, undo history,
/// file IO, UI state, and optional wgpu GPU texture.
pub struct MyApp {
    pub doc: Document,
    pub tools: ToolConfig,
    pub undo: UndoHistory,
    pub file_io: FileIO,
    pub ui: UIState,
    pub gpu_texture: Option<GpuTexture>,
}

impl MyApp {
    /// Create a new `MyApp`, initializing the wgpu GPU texture for partial uploads.
    ///
    /// Falls back to the egui-managed texture path (full-buffer `tex.set()`)
    /// when wgpu render state is unavailable (e.g. Glow backend).
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        use std::sync::mpsc;
        let (dialog_sender, dialog_receiver) = mpsc::channel();
        let (save_result_sender, save_result_receiver) = mpsc::channel();
        let canvas = Canvas::default();
        let pixel_count = (canvas.width * canvas.height) as usize;

        let project_dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORG, APP_NAME).expect(
            "Couldn't resolve app dir"
        );
        let data_dir = project_dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&data_dir).expect("Couldn't create data dir");
        std::fs::create_dir_all(&data_dir.join("autosaves")).expect("Couldn't create autosave dir");

        let gpu_texture = cc.wgpu_render_state.as_ref().map(|rs| {
            let w = canvas.width;
            let h = canvas.height;
            let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
            let texture = rs.device.create_texture(&wgpu::TextureDescriptor {
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
            let mut renderer = rs.renderer.write().unwrap();
            let texture_id = renderer.register_native_texture(
                &rs.device,
                &view,
                wgpu::FilterMode::Linear,
            );
            GpuTexture {
                texture,
                texture_id,
                device: rs.device.clone(),
                queue: rs.queue.clone(),
            }
        });

        Self {
            doc: Document::new(canvas),
            tools: ToolConfig::default(),
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
        // Poll dialog results and save results before anything else.
        self.file_io.poll_dialog_results(&mut self.doc, &mut self.undo, &mut self.ui);
        self.file_io.poll_save_results(&mut self.doc, &mut self.ui);

        if !ui.ctx().input(|i| i.viewport().focused.unwrap_or(true)) {
            std::thread::sleep(std::time::Duration::from_millis(UNFOCUSED_SLEEP_MS));
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
                self.ui.render_state = RenderState::ActiveWake(
                    duration.saturating_sub(predicted_delta_time)
                );
            }
            RenderState::IdleThrottled => {
                ui.request_repaint_after(predicted_delta_time * REPAINT_DELAY_MULT);
            }
            RenderState::UnfocusedFrozen => {
                self.ui.render_state = RenderState::IdleThrottled;
                return;
            }
        }

        // Render layers to texture if needed
        if self.doc.canvas.render_next_frame || self.doc.canvas.rendered_layers.is_none() {
            self.doc.render_to_texture(ui);
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
                        for &(label, w, h) in NEW_CANVAS_PRESETS {
                            if ui.button(format!("{label}\n{w}×{h}")).clicked() {
                                self.ui.new_canvas_width = w;
                                self.ui.new_canvas_height = h;
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
                            self.doc.replace_canvas(canvas, &mut self.undo);
                            self.tools.previous_tool = None;
                            self.tools.previous_cursor_position = None;
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

        if is_quitting {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Autosave every AUTOSAVE_INTERVAL_MINS minutes, but only if the canvas has been modified.
        if
            self.doc.dirty_since_last_autosave &&
            self.ui.time_elapsed.saturating_sub(self.ui.last_autosave_time) >= Duration::from_mins(AUTOSAVE_INTERVAL_MINS)
        {
            self.ui.last_autosave_time = self.ui.time_elapsed;
            self.ui.times_autosaved += 1;
            self.file_io.trigger_async_save(&self.doc, SaveKind::Autosave);
        }
    }
}
