use std::{ collections::VecDeque, time::Duration };

use eframe::egui::{ self, Color32, Panel };
// use serde::{ Deserialize, Serialize };

use crate::canvas::{ Canvas, CurrentTool, RenderState };
use crate::pixel;
use crate::undo::Stroke;

impl MyApp {
    #[inline(always)]
    pub fn push_stroke(&mut self, mut stroke: Stroke) {
        self.stroke_stack.truncate(self.stroke_stack.len() - self.redo_index);
        if self.stroke_stack.len() >= 1000 {
            let mut recycled = self.stroke_stack.pop_front().unwrap();
            recycled.pixels.clear();
            recycled.layer_index = stroke.layer_index;
            recycled.width = stroke.width;
            recycled.pixels.extend(stroke.pixels.drain(..));
            self.stroke_stack.push_back(recycled);
        } else {
            self.stroke_stack.push_back(stroke);
        }
        self.redo_index = 0;
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
    pub redo_index: usize, // 0 = most recent stroke, 1 = one before that, etc. If a stroke is made after undoing, redo_index resets to 0 and all strokes above it are removed from the stack.
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            savefile_path: String::new(),
            canvas: Canvas::default(),
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
            bump_allocator: bumpalo::Bump::with_capacity(64 * 1024 * 1024),
            visited: vec![0u32; 3_000_000],
            visited_stamp: 1,
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !ui.ctx().input(|i| i.viewport().focused.unwrap_or(true)) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            self.render_state = RenderState::Frozen;
            return;
        }
        let predicted_delta_time = Duration::from_millis(
            (ui.ctx().input(|i| i.predicted_dt) * 1000.0) as u64
        );

        match self.render_state {
            RenderState::Warm(duration) => {
                self.render_state = RenderState::Warm(
                    duration.saturating_sub(predicted_delta_time)
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

        if
            self.canvas.render_next_frame ||
            self.canvas.rendered_layers.is_none()
            // &&
            // self.render_state != RenderState::Frozen
        {
            let size = (self.canvas.width as usize) * (self.canvas.height as usize);

            if self.canvas.output_rgba.len() != size * 4 {
                self.canvas.output_rgba = vec![0; size * 4];
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
                        ui
                            .ctx()
                            .load_texture("rendered_layers", image, egui::TextureOptions::LINEAR)
                    );
                }
            }
        }

        let is_quitting = Panel::top("top").show_inside(ui, |ui| self.show_top_panel(ui)).inner;

        Panel::left("side").show_inside(ui, |ui| self.show_left_panel(ui));

        Panel::right("right").show_inside(ui, |ui| self.show_right_panel(ui));

        egui::CentralPanel::default().show_inside(ui, |ui| self.show_central_panel(ui));

        if is_quitting {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
