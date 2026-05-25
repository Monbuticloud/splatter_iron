use std::time::Duration;

use eframe::egui::{ self, Color32, Rect, Pos2 };
use egui::epaint::StrokeKind;
use egui::os::OperatingSystem;

use crate::app::MyApp;
use crate::canvas::{ self, CurrentTool, RenderState };
use crate::undo::{ undo_stroke, redo_stroke };

impl MyApp {
    #[inline(always)]
    pub fn show_central_panel(&mut self, ui: &mut egui::Ui) {
        if self.canvas.rendered_layers.is_some() {
            self.handle_canvas_interaction(ui);
        }
        if !self.stroke_stack.is_empty() {
            self.handle_undo_redo_ui(ui);
        }
    }

    #[inline(always)]
    fn handle_canvas_interaction(&mut self, ui: &mut egui::Ui) {
        if let Some(tex) = &self.canvas.rendered_layers {
            let avail = ui.available_size();
            let tex_size = tex.size_vec2();

            let scale = (avail.x / tex_size.x).min(avail.y / tex_size.y);
            let draw_size = tex_size * scale;

            let response = ui
                .add(
                    egui::Image
                        ::new(tex)
                        .fit_to_exact_size(draw_size)
                        .sense(egui::Sense::click_and_drag())
                )
                .on_hover_cursor(egui::CursorIcon::Crosshair);

            // Brush preview: semi-transparent filled square + outline at cursor
            if self.show_brush_preview {
                if let Some(hover_pos) = response.hover_pos() {
                    let local = hover_pos - response.rect.min;
                    let uv = egui::vec2(
                        local.x / response.rect.width(),
                        local.y / response.rect.height()
                    );

                    let half_radius = self.radius >> 1;
                    let pixel_x = (uv.x * (self.canvas.width as f32)).floor() as u32;
                    let pixel_y = (uv.y * (self.canvas.height as f32)).floor() as u32;

                    // Canvas-space bounds of the brush square
                    let preview_start_x = pixel_x.saturating_sub(half_radius) as f32;
                    let preview_end_x =
                        ((pixel_x + half_radius).min(self.canvas.width - 1) as f32) + 1.0;
                    let preview_start_y = pixel_y.saturating_sub(half_radius) as f32;
                    let preview_end_y =
                        ((pixel_y + half_radius).min(self.canvas.height - 1) as f32) + 1.0;

                    // Map to screen space using the scale factor
                    let screen_x =
                        response.rect.min.x +
                        preview_start_x * (draw_size.x / (self.canvas.width as f32));
                    let screen_y =
                        response.rect.min.y +
                        preview_start_y * (draw_size.y / (self.canvas.height as f32));
                    let screen_w =
                        (preview_end_x - preview_start_x) *
                        (draw_size.x / (self.canvas.width as f32));
                    let screen_h =
                        (preview_end_y - preview_start_y) *
                        (draw_size.y / (self.canvas.height as f32));

                    let preview_rect = Rect::from_min_size(
                        Pos2::new(screen_x, screen_y),
                        egui::vec2(screen_w, screen_h)
                    );

                    // Semi-transparent fill
                    let fill_color = Color32::from_rgba_premultiplied(
                        self.current_color.r(),
                        self.current_color.g(),
                        self.current_color.b(),
                        ((self.current_color.a() as f32) * 0.2) as u8
                    );
                    ui.painter().rect_filled(preview_rect, 0.0, fill_color);

                    // Outline
                    ui.painter().rect_stroke(
                        preview_rect,
                        0.0,
                        egui::Stroke::new(1.0, self.current_color),
                        StrokeKind::Middle
                    );
                }
            }

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

                            if self.previous_tool != Some(CurrentTool::SquareTool) {
                                let half_radius = self.radius >> 1;

                                let start_x = pixel_x.saturating_sub(half_radius);
                                let end_x = (pixel_x + half_radius).min(self.canvas.width - 1);

                                let start_y = pixel_y.saturating_sub(half_radius);
                                let end_y = (pixel_y + half_radius).min(self.canvas.height - 1);

                                let stroke = canvas::draw_square(
                                    start_x,
                                    start_y,
                                    end_x,
                                    end_y,
                                    &mut self.canvas,
                                    self.current_color,
                                    self.current_layer
                                );
                                self.push_stroke(stroke);
                            } else if let Some((past_x, past_y)) = self.previous_cursor_position {
                                let stroke = canvas::draw_square_line(
                                    past_x,
                                    past_y,
                                    pixel_x,
                                    pixel_y,
                                    self.radius,
                                    &mut self.canvas,
                                    self.current_color,
                                    self.current_layer,
                                    &self.bump_allocator
                                );
                                self.push_stroke(stroke);
                            }
                        }
                        CurrentTool::CircleTool => {
                            todo!();
                        }
                        CurrentTool::SquareEraserTool => {
                            self.canvas.render_next_frame = true;

                            if self.previous_tool != Some(CurrentTool::SquareEraserTool) {
                                let half_radius = self.radius >> 1;

                                let start_x = pixel_x.saturating_sub(half_radius);
                                let end_x = (pixel_x + half_radius).min(self.canvas.width - 1);

                                let start_y = pixel_y.saturating_sub(half_radius);
                                let end_y = (pixel_y + half_radius).min(self.canvas.height - 1);

                                let stroke = canvas::draw_square(
                                    start_x,
                                    start_y,
                                    end_x,
                                    end_y,
                                    &mut self.canvas,
                                    Color32::TRANSPARENT,
                                    self.current_layer
                                );
                                self.push_stroke(stroke);
                            } else if let Some((past_x, past_y)) = self.previous_cursor_position {
                                let stroke = canvas::draw_square_line(
                                    past_x,
                                    past_y,
                                    pixel_x,
                                    pixel_y,
                                    self.radius,
                                    &mut self.canvas,
                                    Color32::TRANSPARENT,
                                    self.current_layer,
                                    &self.bump_allocator
                                );
                                self.push_stroke(stroke);
                            }
                        }
                        CurrentTool::CircleEraserTool => {
                            todo!();
                        }
                    }
                    self.previous_tool = Some(self.current_tool);
                    self.previous_cursor_position = Some((pixel_x, pixel_y));
                }
            } else {
                self.previous_tool = None;
                self.previous_cursor_position = None;
            }
        }
    }

    #[inline(always)]
    fn handle_undo_redo_ui(&mut self, ui: &mut egui::Ui) {
        // Platform-aware modifier key label
        let mod_prefix = if ui.ctx().os() == OperatingSystem::Mac { "⌘" } else { "Ctrl+" };

        let undo_btn = ui.button(format!("Undo  {mod_prefix}Z"));
        let redo_btn = ui.button(format!("Redo  {mod_prefix}Shift+Z"));

        // Undo: cmd+Z or Undo button
        if
            self.redo_index < self.stroke_stack.len() &&
            (ui.input(
                |i| i.key_pressed(egui::Key::Z) && i.modifiers.command && !i.modifiers.shift
            ) || undo_btn.clicked())
        {
            let count = self.undo_redo_strength.min(self.stroke_stack.len() - self.redo_index);
            for _ in 0..count {
                let idx = self.stroke_stack.len() - 1 - self.redo_index;
                undo_stroke(&mut self.canvas, &self.stroke_stack[idx]);
                self.redo_index += 1;
            }
            self.canvas.render_next_frame = true;
        }

        // Redo: cmd+shift+Z, cmd+Y, or Redo button
        if
            self.redo_index > 0 &&
            (ui.input(
                |i| i.key_pressed(egui::Key::Z) && i.modifiers.command && i.modifiers.shift
            ) ||
                ui.input(|i| i.key_pressed(egui::Key::Y) && i.modifiers.command) ||
                redo_btn.clicked())
        {
            let count = self.undo_redo_strength.min(self.redo_index);
            for _ in 0..count {
                let idx = self.stroke_stack.len() - self.redo_index;
                self.redo_index -= 1;
                redo_stroke(&mut self.canvas, &self.stroke_stack[idx]);
            }
            self.canvas.render_next_frame = true;
        }
    }
}
