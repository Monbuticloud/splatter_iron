use std::time::Duration;

use eframe::egui::{ self, Color32 };

use crate::app::MyApp;
use crate::canvas::{ self, CurrentTool, RenderState };
use crate::pixel;
use crate::undo::{ self, Stroke, StrokePixel, undo_stroke, redo_stroke };

impl MyApp {
    #[inline(always)]
    pub fn show_central_panel(&mut self, ui: &mut egui::Ui) {
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
                                let half = self.radius >> 1;

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
                                self.stroke_stack.truncate(
                                    self.stroke_stack.len() - self.redo_index
                                );
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
                                self.stroke_stack.truncate(
                                    self.stroke_stack.len() - self.redo_index
                                );
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
                                let half = self.radius >> 1;

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
                                self.stroke_stack.truncate(
                                    self.stroke_stack.len() - self.redo_index
                                );
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
                                self.stroke_stack.truncate(
                                    self.stroke_stack.len() - self.redo_index
                                );
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

        if self.stroke_stack.len() > 1 {
            let undo_button = ui.button("Undo");

            if
                self.redo_index < self.stroke_stack.len() &&
                (ui.input(
                    |i| i.key_pressed(egui::Key::Z) && i.modifiers.command && i.modifiers.shift
                ) || undo_button.clicked())
            {
                self.redo_index -= 1;
                // redo action
                let stroke = &self.stroke_stack[self.stroke_stack.len() - self.redo_index];
                redo_stroke(&mut self.canvas, stroke);
                self.canvas.render_next_frame = true;
            }

            if
                (ui.input(|i| {
                    (i.key_pressed(egui::Key::Y) && i.modifiers.command) ||
                        (i.key_pressed(egui::Key::Z) && i.modifiers.command && i.modifiers.shift)
                }) || undo_button.clicked()) &&
                self.redo_index < self.stroke_stack.len()
            {
                self.redo_index += 1;
                let stroke = &self.stroke_stack[self.stroke_stack.len() - self.redo_index];
                undo_stroke(&mut self.canvas, stroke);
                self.canvas.render_next_frame = true;
            }
        }
    }
}
