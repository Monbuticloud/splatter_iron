use std::{ collections::VecDeque, path::PathBuf, sync::mpsc, time::Duration };

use eframe::egui::{ self, Color32, Panel };

use crate::canvas::{ Canvas, CurrentTool, RenderState };
use crate::pixel::{ self, BYTES_PER_PIXEL as RGBA_CHANNELS };
use crate::undo::Stroke;
use directories::ProjectDirs;
use chrono::Local;

// --- App identity constants ---
pub(crate) const APP_QUALIFIER: &str = "com";
pub(crate) const APP_ORG: &str = "Monbuticloud";
pub(crate) const APP_NAME: &str = "SplatterIron";

// --- Canvas & save file constants ---
const MAX_STROKE_STACK: usize = 1000;
const CANVAS_EXT: &str = ".splattercanvas";
const FILE_FILTER_NAME: &str = "SplatterCanvas";
const DEFAULT_CANVAS_NAME: &str = "canvas.splattercanvas";
const TEXTURE_NAME: &str = "rendered_layers";

// --- Autosave constants ---
const AUTOSAVE_DIR: &str = "autosaves";
const AUTOSAVE_DATE_FMT: &str = "%Y-%m-%d_%H-%M-%S";
const AUTOSAVE_INTERVAL_MINS: u64 = 2;

// --- Performance constants ---
const BUMP_ALLOCATOR_CAPACITY: usize = 32 * 1024 * 1024;
const UNFOCUSED_SLEEP_MS: u64 = 50;
const SECONDS_TO_MILLIS: f32 = 1000.0;
const REPAINT_DELAY_MULT: u32 = 5;

// --- Image import extensions ---
const IMPORT_EXTENSIONS: &[&str] = &[
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

pub(crate) struct ExportInfo {
    pub extensions: &'static [&'static str],
    pub fmt: image::ImageFormat,
}

/// Lookup table for all supported export formats.
pub(crate) const EXPORT_FORMATS: &[(&str, ExportInfo)] = &[
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

/// A file-dialog action queued for execution on a background thread.
/// The result is received via channel at the start of a future frame.
#[derive(Clone, Copy)]
pub(crate) enum PendingFileAction {
    Load,
    Save,
    Import,
    Export(usize), // index into EXPORT_FORMATS
}

/// Message sent back from the file-dialog thread to the UI thread.
pub(crate) enum DialogResult {
    Picked(PathBuf),
}

/// Distinguishes an autosave from a manual save in the async save pipeline.
pub(crate) enum SaveKind {
    Autosave,
    ManualSave(PathBuf),
}

/// Result sent back via channel when an async save completes.
pub(crate) enum SaveResult {
    /// Autosave finished (path is logged, nothing to update in app state).
    Autosave,
    /// Manual save finished — store the path.
    ManualSave(PathBuf),
    /// Save failed with an error message.
    Failed(String),
}

impl MyApp {
    pub fn push_stroke(&mut self, mut stroke: Stroke) {
        self.stroke_stack.truncate(self.stroke_stack.len() - self.redo_index);
        if self.stroke_stack.len() >= MAX_STROKE_STACK {
            let mut recycled = self.stroke_stack.pop_front().unwrap();
            recycled.layer_index = stroke.layer_index;
            recycled.width = stroke.width;
            std::mem::swap(&mut recycled.pixels, &mut stroke.pixels);
            self.stroke_stack.push_back(recycled);
        } else {
            self.stroke_stack.push_back(stroke);
        }
        self.redo_index = 0;
        self.dirty_since_last_autosave = true;
    }

    #[inline(always)]
    pub(crate) fn next_stamp(&mut self) -> u32 {
        self.visited_stamp = self.visited_stamp.wrapping_add(1);
        if self.visited_stamp == 0 {
            self.visited.fill(0);
            self.visited_stamp = 1;
        }
        self.visited_stamp
    }

    /// Resize the visited-stamp vec to match `pixel_count`.
    pub(crate) fn resize_visited(&mut self, pixel_count: usize) {
        if self.visited.len() < pixel_count {
            self.visited = vec![0u32; pixel_count];
        }
        self.visited_stamp = 1;
    }

    /// Replace the canvas and reset associated state.
    pub(crate) fn replace_canvas(&mut self, canvas: Canvas) {
        self.canvas = canvas;
        self.savefile_path.clear();
        self.stroke_stack.clear();
        self.redo_index = 0;
        self.pending_layer_for_deletion = None;
        self.previous_tool = None;
        self.previous_cursor_position = None;
        self.canvas.render_next_frame = true;
        self.resize_visited((self.canvas.width * self.canvas.height) as usize);
    }

    /// Render current layers into the shared texture (GPU).
    fn render_to_texture(&mut self, ui: &egui::Ui) {
        let pixel_count = (self.canvas.width as usize) * (self.canvas.height as usize);

        if self.canvas.output_rgba.len() != pixel_count * RGBA_CHANNELS {
            self.canvas.output_rgba = vec![0; pixel_count * RGBA_CHANNELS];
        }
        self.canvas.render_next_frame = false;

        let layer_slices: Vec<&[Color32]> = self.canvas.pixels
            .iter()
            .map(|l| l.pixels.as_slice())
            .collect();
        pixel::blend_layers(&layer_slices, &mut self.canvas.output_rgba);
        let image = egui::ColorImage::from_rgba_premultiplied(
            [self.canvas.width as usize, self.canvas.height as usize],
            &self.canvas.output_rgba
        );

        match &mut self.canvas.rendered_layers {
            Some(tex) => {
                tex.set(image, egui::TextureOptions::LINEAR);
            }
            None => {
                self.canvas.rendered_layers = Some(
                    ui.ctx().load_texture(TEXTURE_NAME, image, egui::TextureOptions::LINEAR)
                );
            }
        }
    }

    /// Queue a file-dialog action and spawn the dialog on a background thread.
    /// The dialog runs via `dispatch_sync` to the main thread from a background
    /// thread, which avoids macOS winit re-entrancy panics.
    pub(crate) fn queue_file_action(&mut self, action: PendingFileAction) {
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

    /// Check for results from background file-dialog threads and process them.
    /// Called once per frame right before egui layout.
    pub(crate) fn poll_dialog_results(&mut self) {
        use crate::files;
        use std::path::Path;

        while let Ok(result) = self.dialog_receiver.try_recv() {
            match result {
                DialogResult::Picked(path) => {
                    let pending = match self.pending_file_action.take() {
                        Some(a) => a,
                        None => {
                            continue;
                        }
                    };
                    match pending {
                        PendingFileAction::Save => {
                            let path_str = path.display().to_string();
                            let savepath = if path_str.ends_with(CANVAS_EXT) {
                                path
                            } else {
                                PathBuf::from(format!("{path_str}{CANVAS_EXT}"))
                            };
                            self.trigger_async_save(SaveKind::ManualSave(savepath));
                        }
                        PendingFileAction::Load => {
                            match files::load_data_from_file(&path) {
                                Ok(data) => {
                                    match files::load_app_from_data(&data) {
                                        Ok(canvas) => {
                                            let save_path = path.display().to_string();
                                            self.replace_canvas(canvas);
                                            self.savefile_path = save_path;
                                        }
                                        Err(e) => eprintln!("Failed to load canvas: {e}"),
                                    }
                                }
                                Err(e) => eprintln!("Failed to read file: {e}"),
                            }
                        }
                        PendingFileAction::Import => {
                            match files::import_image_as_canvas(&path) {
                                Ok(canvas) => self.replace_canvas(canvas),
                                Err(e) => eprintln!("Import failed: {e}"),
                            }
                        }
                        PendingFileAction::Export(idx) => {
                            if self.canvas.output_rgba.is_empty() {
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
                                let Err(e) = files::export_as_image(
                                    &self.canvas.output_rgba,
                                    self.canvas.width,
                                    self.canvas.height,
                                    Path::new(&path_str),
                                    info.fmt
                                )
                            {
                                eprintln!("Export failed: {e}");
                            }
                        }
                    }
                }
            }
        }
    }

    /// Spawn a background thread to serialise + write the canvas to disk.
    /// The thread clones the current canvas snapshot and processes it off the UI thread.
    fn trigger_async_save(&mut self, kind: SaveKind) {
        let canvas = self.canvas.clone();
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

    /// Save to the current `savefile_path` (async). No-op if path is empty.
    pub(crate) fn save_to_current_path(&mut self) {
        if !self.savefile_path.is_empty() {
            self.trigger_async_save(SaveKind::ManualSave(PathBuf::from(&self.savefile_path)));
        }
    }

    /// Poll for completed async saves and update app state accordingly.
    pub(crate) fn poll_save_results(&mut self) {
        while let Ok(result) = self.save_result_receiver.try_recv() {
            match result {
                SaveResult::Autosave => {
                    self.dirty_since_last_autosave = false;
                }
                SaveResult::ManualSave(path) => {
                    self.savefile_path = path.display().to_string();
                    self.canvas.render_next_frame = true;
                }
                SaveResult::Failed(msg) => {
                    eprintln!("Save failed: {msg}");
                }
            }
        }
    }
}

pub struct MyApp {
    pub savefile_path: String,
    pub current_tool: CurrentTool,
    pub current_color: Color32,
    pub current_layer: usize,
    pub previous_tool: Option<CurrentTool>,
    pub previous_cursor_position: Option<(u32, u32)>,
    pub radius: u32,
    pub canvas: Canvas,
    pub render_state: RenderState,
    pub pending_layer_for_deletion: Option<usize>,
    pub undo_redo_steps_multiplier: usize,
    pub show_brush_preview: bool,
    pub bump_allocator: bumpalo::Bump,
    pub visited: Vec<u32>,
    pub visited_stamp: u32,
    pub stroke_stack: VecDeque<Stroke>,
    pub redo_index: usize,
    pub pending_file_action: Option<PendingFileAction>,
    pub dialog_sender: mpsc::Sender<DialogResult>,
    pub dialog_receiver: mpsc::Receiver<DialogResult>,

    pub app_local_data_directory: PathBuf,
    pub time_elapsed: std::time::Duration,
    pub times_autosaved: u32,

    /// Set to true on any stroke/edit, cleared after autosave completes.
    pub dirty_since_last_autosave: bool,
    /// Channel for async save results.
    pub save_result_sender: mpsc::Sender<SaveResult>,
    pub save_result_receiver: mpsc::Receiver<SaveResult>,
}

impl Default for MyApp {
    fn default() -> Self {
        let (dialog_sender, dialog_receiver) = mpsc::channel();
        let (save_result_sender, save_result_receiver) = mpsc::channel();
        let canvas = Canvas::default();
        let pixel_count = (canvas.width * canvas.height) as usize;

        let project_dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORG, APP_NAME).expect(
            "Couldn't resolve app dir"
        );

        let data_dir = project_dirs.data_dir().to_path_buf();
        let autosave_dir = data_dir.join(AUTOSAVE_DIR);

        std::fs::create_dir_all(&data_dir).expect("Couldn't create data dir");
        std::fs::create_dir_all(&autosave_dir).expect("Couldn't create autosave dir");

        Self {
            savefile_path: String::new(),
            canvas,
            render_state: RenderState::IdleThrottled,
            current_tool: CurrentTool::SquareTool,
            current_color: Color32::from_rgba_premultiplied(255, 255, 255, 255),
            current_layer: 0,
            radius: 100,
            pending_layer_for_deletion: None,
            previous_tool: None,
            previous_cursor_position: None,
            stroke_stack: VecDeque::new(),
            redo_index: 0,
            undo_redo_steps_multiplier: 5,
            show_brush_preview: true,
            bump_allocator: bumpalo::Bump::with_capacity(BUMP_ALLOCATOR_CAPACITY),
            visited: vec![0u32; pixel_count],
            visited_stamp: 1,
            pending_file_action: None,
            dialog_sender,
            dialog_receiver,
            app_local_data_directory: data_dir,
            time_elapsed: std::time::Duration::ZERO,
            times_autosaved: 0,
            dirty_since_last_autosave: false,
            save_result_sender,
            save_result_receiver,
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Poll dialog results and save results before anything else.
        self.poll_dialog_results();
        self.poll_save_results();

        if !ui.ctx().input(|i| i.viewport().focused.unwrap_or(true)) {
            std::thread::sleep(std::time::Duration::from_millis(UNFOCUSED_SLEEP_MS));
            self.render_state = RenderState::UnfocusedFrozen;
            return;
        }
        let predicted_delta_time = Duration::from_millis(
            (ui.ctx().input(|i| i.predicted_dt) * SECONDS_TO_MILLIS) as u64
        );
        let real_delta_time = Duration::from_millis(
            (ui.ctx().input(|i| i.stable_dt) * SECONDS_TO_MILLIS) as u64
        );

        self.time_elapsed += real_delta_time;

        match self.render_state {
            RenderState::ActiveWake(duration) => {
                self.render_state = RenderState::ActiveWake(
                    duration.saturating_sub(predicted_delta_time)
                );
            }
            RenderState::IdleThrottled => {
                ui.request_repaint_after(predicted_delta_time * REPAINT_DELAY_MULT);
            }
            RenderState::UnfocusedFrozen => {
                self.render_state = RenderState::IdleThrottled;
                return;
            }
        }

        self.bump_allocator.reset();

        // Render layers to texture if needed
        if self.canvas.render_next_frame || self.canvas.rendered_layers.is_none() {
            self.render_to_texture(ui);
        }

        let is_quitting = Panel::top("top").show_inside(ui, |ui| self.show_top_panel(ui)).inner;

        Panel::left("side").show_inside(ui, |ui| self.show_left_panel(ui));

        Panel::right("right").show_inside(ui, |ui| self.show_right_panel(ui));

        egui::CentralPanel::default().show_inside(ui, |ui| self.show_central_panel(ui));

        if is_quitting {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Autosave every AUTOSAVE_INTERVAL_MINS minutes, but only if the canvas has been modified.
        if
            self.dirty_since_last_autosave &&
            self.times_autosaved * Duration::from_mins(AUTOSAVE_INTERVAL_MINS) < self.time_elapsed
        {
            self.times_autosaved += 1;
            self.trigger_async_save(SaveKind::Autosave);
        }
    }
}
