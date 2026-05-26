use std::{ collections::VecDeque, time::Duration };

use eframe::egui::{ self, Color32, Panel };

use crate::canvas::{ Canvas, CurrentTool, RenderState };
use crate::pixel;
use crate::undo::Stroke;

/// A file-dialog action queued for execution at the start of the next frame.
/// This avoids macOS winit re-entrancy panics by running native modals
/// *between* frames rather than inside an active event handler.
pub(crate) enum PendingFileAction {
    Load,
    Save,
    Import,
    Export { extensions: &'static [&'static str], fmt: image::ImageFormat },
}

impl MyApp {
    pub fn push_stroke(&mut self, mut stroke: Stroke) {
        self.stroke_stack.truncate(self.stroke_stack.len() - self.redo_index);
        if self.stroke_stack.len() >= 1000 {
            let mut recycled = self.stroke_stack.pop_front().unwrap();
            recycled.layer_index = stroke.layer_index;
            recycled.width = stroke.width;
            std::mem::swap(&mut recycled.pixels, &mut stroke.pixels);
            self.stroke_stack.push_back(recycled);
        } else {
            self.stroke_stack.push_back(stroke);
        }
        self.redo_index = 0;
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
    /// Call after canvas dimensions change (New, Load, Import).
    pub(crate) fn resize_visited(&mut self, pixel_count: usize) {
        if self.visited.len() < pixel_count {
            self.visited = vec![0u32; pixel_count];
        }
        self.visited_stamp = 1;
    }

    /// Replace the canvas and reset associated state.
    /// Used by Load, New, and Import operations.
    pub(crate) fn replace_canvas(&mut self, canvas: Canvas) {
        self.canvas = canvas;
        self.savefile_path.clear();
        self.stroke_stack.clear();
        self.redo_index = 0;
        self.pending_delete_layer = None;
        self.previous_tool = None;
        self.previous_cursor_position = None;
        self.canvas.render_next_frame = true;
        self.resize_visited((self.canvas.width * self.canvas.height) as usize);
    }

    /// Render current layers into the shared texture (GPU).
    /// Call this once per frame, before the panels are drawn.
    fn render_to_texture(&mut self, ui: &egui::Ui) {
        let pixel_count = (self.canvas.width as usize) * (self.canvas.height as usize);

        if self.canvas.output_rgba.len() != pixel_count * 4 {
            self.canvas.output_rgba = vec![0; pixel_count * 4];
        }
        self.canvas.render_next_frame = false;

        let layer_slices: Vec<&[Color32]> = self.canvas.pixels
            .iter()
            .map(|l| l.pixels.as_slice())
            .collect();
        pixel::blend_layers(&layer_slices, &mut self.canvas.output_rgba);
        let image = egui::ColorImage::from_rgba_premultiplied(
            [self.canvas.width as usize, self.canvas.height as usize],
            &self.canvas.output_rgba,
        );

        match &mut self.canvas.rendered_layers {
            Some(tex) => {
                tex.set(image, egui::TextureOptions::LINEAR);
            }
            None => {
                self.canvas.rendered_layers = Some(
                    ui.ctx()
                        .load_texture("rendered_layers", image, egui::TextureOptions::LINEAR),
                );
            }
        }
    }

    /// Process any deferred file-dialog action at the safe point between frames.
    pub(crate) fn handle_pending_file_action(&mut self) {
        let action = match self.pending_file_action.take() {
            Some(a) => a,
            None => return,
        };

        use crate::files;
        use std::path::Path;

        match action {
            PendingFileAction::Save => {
                if self.savefile_path.is_empty() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("SplatterCanvas", &["splattercanvas"])
                        .set_file_name("canvas.splattercanvas")
                        .save_file()
                    {
                        let path_str = path.display().to_string();
                        self.savefile_path = if path_str.ends_with(".splattercanvas") {
                            path_str
                        } else {
                            format!("{}.splattercanvas", path_str)
                        };
                    }
                }
                if !self.savefile_path.is_empty() {
                    if let Err(e) = files::save_canvas(self) {
                        eprintln!("Save failed: {e}");
                    }
                }
                self.canvas.render_next_frame = true;
            }
            PendingFileAction::Load => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("SplatterCanvas", &["splattercanvas"])
                    .pick_file()
                {
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
            }
            PendingFileAction::Import => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        "Images",
                        &["avif", "png", "jpg", "jpeg", "webp", "gif", "tiff", "tif",
                          "tga", "ico", "pnm", "pgm", "ppm", "pbm", "pam", "qoi", "exr", "hdr", "ff"],
                    )
                    .pick_file()
                {
                    match files::import_image_as_canvas(&path) {
                        Ok(canvas) => self.replace_canvas(canvas),
                        Err(e) => eprintln!("Import failed: {e}"),
                    }
                }
            }
            PendingFileAction::Export { extensions, fmt } => {
                if self.canvas.output_rgba.is_empty() {
                    return;
                }
                let default_ext = extensions[0];
                let default_name = format!("export.{default_ext}");
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(extensions[0], extensions)
                    .set_file_name(&default_name)
                    .save_file()
                {
                    let path_str = path.display().to_string();
                    let path_str = if extensions.iter().any(|ext| path_str.ends_with(ext)) {
                        path_str
                    } else {
                        format!("{path_str}.{default_ext}")
                    };
                    if let Err(e) = files::export_as_image(
                        &self.canvas.output_rgba,
                        self.canvas.width,
                        self.canvas.height,
                        Path::new(&path_str),
                        fmt,
                    ) {
                        eprintln!("Export failed: {e}");
                    }
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
    pub pending_delete_layer: Option<usize>,
    pub undo_redo_strength: usize,
    pub show_brush_preview: bool,
    pub bump_allocator: bumpalo::Bump,
    pub visited: Vec<u32>,
    pub visited_stamp: u32,
    pub stroke_stack: VecDeque<Stroke>,
    pub redo_index: usize,
    pub pending_file_action: Option<PendingFileAction>,
}

impl Default for MyApp {
    fn default() -> Self {
        let canvas = Canvas::default();
        let pixel_count = (canvas.width * canvas.height) as usize;
        Self {
            savefile_path: String::new(),
            canvas,
            render_state: RenderState::Cold,
            current_tool: CurrentTool::SquareTool,
            current_color: Color32::from_rgba_premultiplied(255, 255, 255, 255),
            current_layer: 0,
            radius: 100,
            pending_delete_layer: None,
            previous_tool: None,
            previous_cursor_position: None,
            stroke_stack: VecDeque::new(),
            redo_index: 0,
            undo_redo_strength: 5,
            show_brush_preview: true,
            bump_allocator: bumpalo::Bump::with_capacity(32 * 1024 * 1024),
            visited: vec![0u32; pixel_count],
            visited_stamp: 1,
            pending_file_action: None,
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Process any deferred file dialog before anything else
        // (between frames, safe for native macOS modals).
        self.handle_pending_file_action();

        if !ui.ctx().input(|i| i.viewport().focused.unwrap_or(true)) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            self.render_state = RenderState::Frozen;
            return;
        }
        let predicted_delta_time = Duration::from_millis(
            (ui.ctx().input(|i| i.predicted_dt) * 1000.0) as u64,
        );

        match self.render_state {
            RenderState::Warm(duration) => {
                self.render_state = RenderState::Warm(
                    duration.saturating_sub(predicted_delta_time),
                );
            }
            RenderState::Cold => {
                ui.request_repaint_after(predicted_delta_time * 5);
            }
            RenderState::Frozen => {
                self.render_state = RenderState::Cold;
                return;
            }
        }

        self.bump_allocator.reset();

        // Render layers to texture if needed
        if self.canvas.render_next_frame || self.canvas.rendered_layers.is_none() {
            self.render_to_texture(ui);
        }

        let is_quitting =
            Panel::top("top").show_inside(ui, |ui| self.show_top_panel(ui)).inner;

        Panel::left("side").show_inside(ui, |ui| self.show_left_panel(ui));

        Panel::right("right").show_inside(ui, |ui| self.show_right_panel(ui));

        egui::CentralPanel::default().show_inside(ui, |ui| self.show_central_panel(ui));

        if is_quitting {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}