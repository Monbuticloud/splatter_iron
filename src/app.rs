use std::{ time::Duration };

use eframe::egui::{ self, Color32, Frame, Panel };
// use serde::{ Deserialize, Serialize };

use crate::canvas::{ self, Canvas, CurrentTool, Layer, RenderState };
use crate::undo::*;

pub struct MyApp {
    pub savefile_path: String,
    pub current_tool: CurrentTool,
    pub current_color: Color32,
    pub current_layer: usize,
    pub past_tool: Option<CurrentTool>,
    pub past_position: Option<(u32, u32)>,
    pub radius: u32,
    pub canvas: Canvas,
    pub input_color_text: String,
    pub input_radius_text: String,
    pub render_state: RenderState,
    pub pending_delete_layer: Option<usize>,

    pub stroke_stack: Vec<Stroke>,
    pub redo_index: usize, // 0 = most recent stroke, 1 = one before that, etc. If a stroke is made after undoing, redo_index resets to 0 and all strokes above it are removed from the stack.
}

enum LayerAction {
    Delete(usize),
    MoveUp(usize),
    MoveDown(usize),
    Select(usize),
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
            input_color_text: String::from("(255, 255, 255, 255)"),
            input_radius_text: String::from("100"),
            past_tool: None,
            past_position: None,
            stroke_stack: Vec::new(),
            redo_index: 0,
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if !ui.ctx().input(|i| i.viewport().focused.unwrap_or(true)) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            self.render_state = RenderState::Frozen;
            return;
        }
        let dt = Duration::from_millis((ui.ctx().input(|i| i.predicted_dt) * 1000.0) as u64);

        match self.render_state {
            RenderState::Warm(duration) => {
                self.render_state = RenderState::Warm(duration.saturating_sub(dt));
            }
            RenderState::Cold => {
                ui.request_repaint_after(dt * 5);
            }
            RenderState::Frozen => {
                self.render_state = RenderState::Cold;
                return;
            }
        }

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

            if self.canvas.output_rgba.len() != size * 4 {
                self.canvas.output_rgba = vec![0; size * 4];
            }

            canvas::composite_layers_parallel_rgba(
                &self.canvas.pixels,
                &mut self.canvas.output_rgba
            );
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

        let mut is_quitting = false;
        Panel::top("top").show_inside(ui, |top_panel| {
            top_panel.horizontal(|top_panel_alignment| {
                let save_button = top_panel_alignment.button("Save");
                if save_button.clicked() {
                    todo!();
                }

                let load_button = top_panel_alignment.button("Load");
                if load_button.clicked() {
                    todo!();
                }
                let new_button = top_panel_alignment.button("New");
                if new_button.clicked() {
                    todo!();
                }
                let export_button = top_panel_alignment.button("Export").clicked();
                if export_button {
                    todo!();
                }
                let import_button = top_panel_alignment.button("Import");
                if import_button.clicked() {
                    todo!();
                }
                let close_button = top_panel_alignment.button("Close");
                if close_button.clicked() {
                    is_quitting = true;
                }
            });
        });

        Panel::left("side").show_inside(ui, |ui| {
            let square_paint_tool_button = ui.button("Square Tool");
            if square_paint_tool_button.clicked() {
                self.current_tool = CurrentTool::SquareTool;
            }
            let circle_paint_tool_button = ui.button("Circle Tool");
            if circle_paint_tool_button.clicked() {
                self.current_tool = CurrentTool::CircleTool;
            }
            let square_eraser_tool_button = ui.button("Square Eraser");
            if square_eraser_tool_button.clicked() {
                self.current_tool = CurrentTool::SquareEraserTool;
            }
            let circle_eraser_tool_button = ui.button("Circle Eraser");
            if circle_eraser_tool_button.clicked() {
                self.current_tool = CurrentTool::CircleEraserTool;
            }
        });

        let right_panel = Panel::right("right");

        right_panel.show_inside(ui, |ui| {
            ui.label("Settings");

            ui.color_edit_button_srgba(&mut self.current_color);
            ui.add(egui::DragValue::new(&mut self.radius).range(0..=300));
            let current_layer_text = format!("Current Layer: {}", self.current_layer);

            let add_layer_button = ui.button("Add Layer");
            if add_layer_button.clicked() {
                self.canvas.pixels.push(Layer {
                    pixels: vec![Color32::TRANSPARENT; (self.canvas.width * self.canvas.height) as usize],
                });
                // self.canvas.render_next_frame = true;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut status = None;
                for (i, layer) in self.canvas.pixels.iter().enumerate() {
                    let layer_panel = egui::CollapsingHeader
                        ::new(format!("Layer {}", i))
                        .show(ui, |ui| {
                            let delete_button = ui.button("Delete");
                            if delete_button.clicked() {
                                status = Some(LayerAction::Delete(i));
                            }

                            let move_up_button = ui.button("Move Up");
                            if move_up_button.clicked() && i > 0 {
                                // self.canvas.pixels.swap(i, i - 1);
                                status = Some(LayerAction::MoveUp(i));
                            }

                            let move_down_button = ui.button("Move Down");
                            if move_down_button.clicked() && i < self.canvas.pixels.len() - 1 {
                                // self.canvas.pixels.swap(i, i + 1);``
                                status = Some(LayerAction::MoveDown(i));
                            }

                            let select_button = ui.button("Select");
                            if select_button.clicked() {
                                status = Some(LayerAction::Select(i));
                            }
                            if i == self.current_layer {
                                ui.label("Currently Selected");
                            }
                        });
                }
                if let Some(layer_action) = status {
                    match layer_action {
                        LayerAction::Delete(index) => {
                            if self.pending_delete_layer == Some(index) {
                                self.pending_delete_layer = None;
                                self.canvas.pixels.remove(index);
                                self.canvas.render_next_frame = true;
                            } else {
                                self.pending_delete_layer = Some(index);
                            }
                        }
                        LayerAction::MoveUp(index) => {
                            if index > 0 {
                                self.canvas.pixels.swap(index, index - 1);
                                self.canvas.render_next_frame = true;
                                self.current_layer = index - 1;
                            }
                        }
                        LayerAction::MoveDown(index) => {
                            if index < self.canvas.pixels.len() - 1 {
                                self.canvas.pixels.swap(index, index + 1);
                                self.canvas.render_next_frame = true;
                                self.current_layer = index + 1;
                            }
                        }
                        LayerAction::Select(index) => {
                            self.current_layer = index;
                        }
                    }
                }
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(tex) = &self.canvas.rendered_layers {
                let avail = ui.available_size();
                let tex_size = tex.size_vec2();

                let scale = (avail.x / tex_size.x).min(avail.y / tex_size.y);
                let draw_size = tex_size * scale;

                let response = ui.add(
                    egui::Image
                        ::new(tex)
                        .fit_to_exact_size(draw_size)
                        .sense(egui::Sense::click_and_drag())
                );

                if response.hovered() {
                    self.pending_delete_layer = None;
                    self.render_state = RenderState::Warm(Duration::from_millis(550));
                }

                if response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let local = pos - response.rect.min;
                        let uv = egui::vec2(
                            local.x / response.rect.width(),
                            local.y / response.rect.height()
                        );

                        let pixel_x = (uv.x * (self.canvas.width as f32)).floor() as u32;
                        let pixel_y = (uv.y * (self.canvas.height as f32)).floor() as u32;

                        match self.current_tool {
                            CurrentTool::SquareTool => {
                                self.canvas.render_next_frame = true;

                                if self.past_tool != Some(CurrentTool::SquareTool) {
                                    let half = self.radius / 2;

                                    let start_x = pixel_x.saturating_sub(half);
                                    let end_x = (pixel_x + half).min(self.canvas.width - 1);

                                    let start_y = pixel_y.saturating_sub(half);
                                    let end_y = (pixel_y + half).min(self.canvas.height - 1);

                                    let stroke = canvas::draw_square(
                                        start_x,
                                        start_y,
                                        end_x,
                                        end_y,
                                        &mut self.canvas,
                                        self.current_color,
                                        self.current_layer
                                    );
                                    self.stroke_stack.truncate(self.stroke_stack.len() - self.redo_index);
                                    self.stroke_stack.push(stroke);
                                    self.redo_index = 0;
                                } else if let Some((past_x, past_y)) = self.past_position {
                                    let stroke = canvas::draw_square_line(
                                        past_x,
                                        past_y,
                                        pixel_x,
                                        pixel_y,
                                        self.radius,
                                        &mut self.canvas,
                                        self.current_color,
                                        self.current_layer
                                    );
                                    self.stroke_stack.truncate(self.stroke_stack.len() - self.redo_index);
                                    self.stroke_stack.push(stroke);
                                    self.redo_index = 0;
                                }
                            }
                            CurrentTool::CircleTool => {
                                todo!();
                            }
                            CurrentTool::SquareEraserTool => {
                                self.canvas.render_next_frame = true;

                                if self.past_tool != Some(CurrentTool::SquareEraserTool) {
                                    let half = self.radius / 2;

                                    let start_x = pixel_x.saturating_sub(half);
                                    let end_x = (pixel_x + half).min(self.canvas.width - 1);

                                    let start_y = pixel_y.saturating_sub(half);
                                    let end_y = (pixel_y + half).min(self.canvas.height - 1);

                                    let stroke = canvas::draw_square(
                                        start_x,
                                        start_y,
                                        end_x,
                                        end_y,
                                        &mut self.canvas,
                                        Color32::TRANSPARENT,
                                        self.current_layer
                                    );
                                    self.stroke_stack.truncate(self.stroke_stack.len() - self.redo_index);
                                    self.stroke_stack.push(stroke);
                                    self.redo_index = 0;
                                } else if let Some((past_x, past_y)) = self.past_position {
                                    let stroke = canvas::erase_square_line(
                                        past_x,
                                        past_y,
                                        pixel_x,
                                        pixel_y,
                                        self.radius,
                                        &mut self.canvas,
                                        self.current_layer
                                    );
                                    self.stroke_stack.truncate(self.stroke_stack.len() - self.redo_index);
                                    self.stroke_stack.push(stroke);
                                    self.redo_index = 0;
                                }
                            }
                            CurrentTool::CircleEraserTool => {
                                todo!();
                            }
                        }
                        self.past_tool = Some(self.current_tool);
                        self.past_position = Some((pixel_x, pixel_y));
                    }
                } else {
                    self.past_tool = None;
                    self.past_position = None;
                }
            }
        });

        if is_quitting {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
