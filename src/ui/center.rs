use std::time::Duration;

use eframe::egui::{ self, Color32, Rect, Pos2 };
use egui::epaint::StrokeKind;

use crate::app::MyApp;
use crate::canvas::{ self, CurrentTool, RenderState };
use crate::file_io::PendingFileAction;

const PREVIEW_FILL_ALPHA_FACTOR: f32 = 0.2;
const PREVIEW_STROKE_WIDTH: f32 = 1.0;
const ACTIVE_DURATION_MS: u64 = 550;

impl MyApp {
    /// Render the central canvas panel.
    ///
    /// Only renders if a texture exists (wgpu or fallback). Delegates
    /// interaction handling to `handle_canvas_interaction`.
    pub fn show_central_panel(&mut self, ui: &mut egui::Ui) {
        if self.gpu_texture.is_some() || self.doc.canvas.rendered_layers.is_some() {
            self.handle_canvas_interaction(ui);
        }
    }

    /// Process mouse interaction on the canvas texture.
    ///
    /// Handles brush preview rendering, context menu (Import/Export As/Save As),
    /// drawing strokes with the current tool on drag, and bucket fill on click.
    /// Updates the undo history and marks the document as dirty.
    fn handle_canvas_interaction(&mut self, ui: &mut egui::Ui) {
        let (tex_id, canvas_pixel_size) = if let Some(gpu) = &self.gpu_texture {
            (gpu.texture_id, egui::vec2(
                self.doc.canvas.width as f32,
                self.doc.canvas.height as f32,
            ))
        } else if let Some(tex) = &self.doc.canvas.rendered_layers {
            (tex.id(), tex.size_vec2())
        } else {
            return;
        };

        let avail = ui.available_size();
        let scale = (avail.x / canvas_pixel_size.x).min(avail.y / canvas_pixel_size.y);
        let draw_size = canvas_pixel_size * scale;

        let response = ui
            .add(
                egui::Image
                    ::new((tex_id, canvas_pixel_size))
                    .fit_to_exact_size(draw_size)
                    .sense(egui::Sense::click_and_drag())
            )
            .on_hover_cursor(egui::CursorIcon::Crosshair);

            response.context_menu(|ui| {
                // --- Import ---
                if ui.button("Import").clicked() {
                    self.file_io.pending_file_action = Some(PendingFileAction::Import);
                    ui.ctx().request_repaint();
                    ui.close();
                }

                // --- Export As submenu ---
                ui.menu_button("Export As", |ui| {
                    for (i, &(label, _)) in crate::app::EXPORT_FORMATS.iter().enumerate() {
                        if ui.button(label).clicked() {
                            self.file_io.queue_file_action(PendingFileAction::Export(i));
                            ui.ctx().request_repaint();
                            ui.close();
                        }
                    }
                });

                ui.separator();

                // --- Save As ---
                if ui.button("Save As").clicked() {
                    // Save As always opens a dialog even if savefile_path is set
                    self.file_io.queue_file_action(PendingFileAction::Save);
                    self.doc.savefile_path.clear(); // force dialog
                    ui.ctx().request_repaint();
                    ui.close();
                }
            });

            // Brush preview: outline at cursor (circle outline for circle tools,
            // semi-transparent filled rect + outline for square tools).
            // Bucket Fill has no preview (fills connected regions, not fixed shape).
            if self.tools.show_brush_preview && let Some(hover_pos) = response.hover_pos() {
                    let local = hover_pos - response.rect.min;
                    let uv = egui::vec2(
                        local.x / response.rect.width(),
                        local.y / response.rect.height()
                    );

                    let pixel_x = (uv.x * (self.doc.canvas.width as f32)).floor() as u32;
                    let pixel_y = (uv.y * (self.doc.canvas.height as f32)).floor() as u32;

                    match self.tools.current_tool {
                        CurrentTool::Circle | CurrentTool::CircleEraser => {
                            let center_screen_x =
                                response.rect.min.x +
                                (pixel_x as f32) * (draw_size.x / (self.doc.canvas.width as f32));
                            let center_screen_y =
                                response.rect.min.y +
                                (pixel_y as f32) * (draw_size.y / (self.doc.canvas.height as f32));
                            let screen_radius =
                                (self.tools.radius as f32) *
                                (draw_size.x / (self.doc.canvas.width as f32));

                            ui.painter().circle_stroke(
                                Pos2::new(center_screen_x, center_screen_y),
                                screen_radius,
                                egui::Stroke::new(PREVIEW_STROKE_WIDTH, self.tools.current_color),
                            );
                        }
                        CurrentTool::Square | CurrentTool::SquareEraser => {
                            let half_radius = self.tools.radius;

                            let preview_start_x = pixel_x.saturating_sub(half_radius) as f32;
                            let preview_end_x =
                                ((pixel_x + half_radius).min(self.doc.canvas.width - 1) as f32) + 1.0;
                            let preview_start_y = pixel_y.saturating_sub(half_radius) as f32;
                            let preview_end_y =
                                ((pixel_y + half_radius).min(self.doc.canvas.height - 1) as f32) + 1.0;

                            // Map to screen space using the scale factor
                            let screen_x =
                                response.rect.min.x +
                                preview_start_x * (draw_size.x / (self.doc.canvas.width as f32));
                            let screen_y =
                                response.rect.min.y +
                                preview_start_y * (draw_size.y / (self.doc.canvas.height as f32));
                            let screen_w =
                                (preview_end_x - preview_start_x) *
                                (draw_size.x / (self.doc.canvas.width as f32));
                            let screen_h =
                                (preview_end_y - preview_start_y) *
                                (draw_size.y / (self.doc.canvas.height as f32));

                            let preview_rect = Rect::from_min_size(
                                Pos2::new(screen_x, screen_y),
                                egui::vec2(screen_w, screen_h)
                            );

                            // Semi-transparent fill (re-premultiply RGB for the reduced alpha)
                            let brush_alpha = self.tools.current_color.a();
                            let fill_color = if brush_alpha == 0 {
                                Color32::TRANSPARENT
                            } else {
                                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                                let preview_alpha =
                                    ((brush_alpha as f32) * PREVIEW_FILL_ALPHA_FACTOR) as u8;
                                Color32::from_rgba_premultiplied(
                                    ((self.tools.current_color.r() as u32 * preview_alpha as u32)
                                        / brush_alpha as u32) as u8,
                                    ((self.tools.current_color.g() as u32 * preview_alpha as u32)
                                        / brush_alpha as u32) as u8,
                                    ((self.tools.current_color.b() as u32 * preview_alpha as u32)
                                        / brush_alpha as u32) as u8,
                                    preview_alpha,
                                )
                            };
                            ui.painter().rect_filled(preview_rect, 0.0, fill_color);

                            // Outline
                            ui.painter().rect_stroke(
                                preview_rect,
                                0.0,
                                egui::Stroke::new(PREVIEW_STROKE_WIDTH, self.tools.current_color),
                                StrokeKind::Middle
                            );
                        }
                        CurrentTool::BucketFill => {}
                    }
                }

            if response.hovered() {
                self.ui.pending_layer_for_deletion = None;
                self.ui.render_state = RenderState::ActiveWake(
                    Duration::from_millis(ACTIVE_DURATION_MS)
                );
            }

            // Bucket fill fires on single click too (not just drag)
            if response.clicked() && self.tools.current_tool == CurrentTool::BucketFill {
                if let Some(pos) = response.interact_pointer_pos() {
                    let local = pos - response.rect.min;
                    let uv = egui::vec2(
                        local.x / response.rect.width(),
                        local.y / response.rect.height()
                    );

                    let pixel_x = (uv.x * (self.doc.canvas.width as f32)).floor() as u32;
                    let pixel_y = (uv.y * (self.doc.canvas.height as f32)).floor() as u32;

                    self.doc.canvas.render_next_frame = true;
                    let stroke = canvas::draw_bucket_fill(
                        pixel_x,
                        pixel_y,
                        &mut self.doc.canvas,
                        self.tools.current_color,
                        self.doc.current_layer,
                        self.tools.alpha_overlay,
                    );
                    self.undo.push_undo(stroke);
                    self.doc.dirty_since_last_autosave = true;
                }
            }

            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let local = pos - response.rect.min;
                    let uv = egui::vec2(
                        local.x / response.rect.width(),
                        local.y / response.rect.height()
                    );

                    let pixel_x = (uv.x * (self.doc.canvas.width as f32)).floor() as u32;
                    let pixel_y = (uv.y * (self.doc.canvas.height as f32)).floor() as u32;

                    match self.tools.current_tool {
                        CurrentTool::Square => {
                            self.doc.canvas.render_next_frame = true;

                            if self.tools.previous_cursor_position.is_none() {
                                if self.tools.alpha_overlay {
                                    self.undo.advance_drag_stamp();
                                    let stamp = self.undo.next_stamp();
                                    let stroke = canvas::draw_square_line(
                                        pixel_x,
                                        pixel_y,
                                        pixel_x,
                                        pixel_y,
                                        self.tools.radius,
                                        &mut self.doc.canvas,
                                        self.tools.current_color,
                                        self.doc.current_layer,
                                        &mut self.undo.visited,
                                        stamp,
                                        true,
                                        &mut self.undo.drag_processed,
                                        self.undo.drag_stamp_val,
                                    );
                                    self.undo.push_undo(stroke);
                                    self.doc.dirty_since_last_autosave = true;
                                } else {
                                    let half_radius = self.tools.radius;

                                    let start_x = pixel_x.saturating_sub(half_radius);
                                    let end_x = (pixel_x + half_radius)
                                        .min(self.doc.canvas.width - 1);

                                    let start_y = pixel_y.saturating_sub(half_radius);
                                    let end_y = (pixel_y + half_radius)
                                        .min(self.doc.canvas.height - 1);

                                    let stroke = canvas::draw_square(
                                        start_x,
                                        start_y,
                                        end_x,
                                        end_y,
                                        &mut self.doc.canvas,
                                        self.tools.current_color,
                                        self.doc.current_layer,
                                        false,
                                    );
                                    self.undo.push_undo(stroke);
                                    self.doc.dirty_since_last_autosave = true;
                                }
                            } else if let Some((past_x, past_y)) =
                                self.tools.previous_cursor_position
                            {
                                let stamp = self.undo.next_stamp();
                                let stroke = canvas::draw_square_line(
                                    past_x,
                                    past_y,
                                    pixel_x,
                                    pixel_y,
                                    self.tools.radius,
                                    &mut self.doc.canvas,
                                    self.tools.current_color,
                                    self.doc.current_layer,
                                    &mut self.undo.visited,
                                    stamp,
                                    self.tools.alpha_overlay,
                                    &mut self.undo.drag_processed,
                                    self.undo.drag_stamp_val,
                                );
                                self.undo.push_undo(stroke);
                                self.doc.dirty_since_last_autosave = true;
                            }
                        }
                        CurrentTool::Circle => {
                            self.doc.canvas.render_next_frame = true;

                            if self.tools.previous_cursor_position.is_none() {
                                if self.tools.alpha_overlay {
                                    self.undo.advance_drag_stamp();
                                    let stamp = self.undo.next_stamp();
                                    let stroke = canvas::draw_circle_line(
                                        pixel_x,
                                        pixel_y,
                                        pixel_x,
                                        pixel_y,
                                        self.tools.radius,
                                        &mut self.doc.canvas,
                                        self.tools.current_color,
                                        self.doc.current_layer,
                                        &mut self.undo.visited,
                                        stamp,
                                        true,
                                        &mut self.undo.drag_processed,
                                        self.undo.drag_stamp_val,
                                    );
                                    self.undo.push_undo(stroke);
                                    self.doc.dirty_since_last_autosave = true;
                                } else {
                                    let stroke = canvas::draw_circle(
                                        pixel_x,
                                        pixel_y,
                                        self.tools.radius,
                                        &mut self.doc.canvas,
                                        self.tools.current_color,
                                        self.doc.current_layer,
                                        false,
                                    );
                                    self.undo.push_undo(stroke);
                                    self.doc.dirty_since_last_autosave = true;
                                }
                            } else if let Some((past_x, past_y)) =
                                self.tools.previous_cursor_position
                            {
                                let stamp = self.undo.next_stamp();
                                let stroke = canvas::draw_circle_line(
                                    past_x,
                                    past_y,
                                    pixel_x,
                                    pixel_y,
                                    self.tools.radius,
                                    &mut self.doc.canvas,
                                    self.tools.current_color,
                                    self.doc.current_layer,
                                    &mut self.undo.visited,
                                    stamp,
                                    self.tools.alpha_overlay,
                                    &mut self.undo.drag_processed,
                                    self.undo.drag_stamp_val,
                                );
                                self.undo.push_undo(stroke);
                                self.doc.dirty_since_last_autosave = true;
                            }
                        }
                        CurrentTool::SquareEraser => {
                            self.doc.canvas.render_next_frame = true;

                            if self.tools.previous_cursor_position.is_none() {
                                let half_radius = self.tools.radius;

                                let start_x = pixel_x.saturating_sub(half_radius);
                                let end_x = (pixel_x + half_radius)
                                    .min(self.doc.canvas.width - 1);

                                let start_y = pixel_y.saturating_sub(half_radius);
                                let end_y = (pixel_y + half_radius)
                                    .min(self.doc.canvas.height - 1);

                        let stroke = canvas::draw_square(
                            start_x,
                            start_y,
                            end_x,
                            end_y,
                            &mut self.doc.canvas,
                            Color32::TRANSPARENT,
                            self.doc.current_layer,
                            false,
                        );
                                self.undo.push_undo(stroke);
                                self.doc.dirty_since_last_autosave = true;
                            } else if let Some((past_x, past_y)) =
                                self.tools.previous_cursor_position
                            {
                                let stamp = self.undo.next_stamp();
                                let stroke = canvas::draw_square_line(
                                    past_x,
                                    past_y,
                                    pixel_x,
                                    pixel_y,
                                    self.tools.radius,
                                    &mut self.doc.canvas,
                                    Color32::TRANSPARENT,
                                    self.doc.current_layer,
                                    &mut self.undo.visited,
                                    stamp,
                                    false,
                                    &mut self.undo.drag_processed,
                                    self.undo.drag_stamp_val,
                                );
                                self.undo.push_undo(stroke);
                                self.doc.dirty_since_last_autosave = true;
                            }
                        }
                        CurrentTool::CircleEraser => {
                            self.doc.canvas.render_next_frame = true;

                            if self.tools.previous_cursor_position.is_none() {
                                let stroke = canvas::draw_circle(
                                    pixel_x,
                                    pixel_y,
                                    self.tools.radius,
                                    &mut self.doc.canvas,
                                    Color32::TRANSPARENT,
                                    self.doc.current_layer,
                                    false,
                                );
                                self.undo.push_undo(stroke);
                                self.doc.dirty_since_last_autosave = true;
                            } else if let Some((past_x, past_y)) =
                                self.tools.previous_cursor_position
                            {
                                let stamp = self.undo.next_stamp();
                                let stroke = canvas::draw_circle_line(
                                    past_x,
                                    past_y,
                                    pixel_x,
                                    pixel_y,
                                    self.tools.radius,
                                    &mut self.doc.canvas,
                                    Color32::TRANSPARENT,
                                    self.doc.current_layer,
                                    &mut self.undo.visited,
                                    stamp,
                                    false,
                                    &mut self.undo.drag_processed,
                                    self.undo.drag_stamp_val,
                                );
                                self.undo.push_undo(stroke);
                                self.doc.dirty_since_last_autosave = true;
                            }
                        }
                        CurrentTool::BucketFill => {
                            self.doc.canvas.render_next_frame = true;
                            let stroke = canvas::draw_bucket_fill(
                                pixel_x,
                                pixel_y,
                                &mut self.doc.canvas,
                                self.tools.current_color,
                                self.doc.current_layer,
                                self.tools.alpha_overlay,
                            );
                            self.undo.push_undo(stroke);
                            self.doc.dirty_since_last_autosave = true;
                        }
                    }
                    self.tools.previous_tool = Some(self.tools.current_tool);
                    self.tools.previous_cursor_position = Some((pixel_x, pixel_y));
                }
            } else {
                self.tools.previous_tool = None;
                self.tools.previous_cursor_position = None;
            }
    }
}
