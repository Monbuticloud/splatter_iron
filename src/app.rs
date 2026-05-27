use std::time::Duration;

use eframe::egui::{ self, Panel };
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
        }
    }
}

/// Top-level application state owned by eframe: document, tools, undo history, file IO, and UI state.
pub struct MyApp {
    pub doc: Document,
    pub tools: ToolConfig,
    pub undo: UndoHistory,
    pub file_io: FileIO,
    pub ui: UIState,
}

impl Default for MyApp {
    /// Create a default `MyApp` with a default canvas, tool config, undo history,
    /// file IO channels, and UI state. Ensures autosave directories exist.
    fn default() -> Self {
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
