//! Central canvas panel: renders the composited texture, handles mouse
//! interaction (brush strokes, eraser, bucket-fill), and applies strokes
//! to the document.

use std::sync::Arc;
use std::time::Duration;

use eframe::egui::Color32;
use eframe::egui::Pos2;
use eframe::egui::Rect;
use eframe::egui::{self};
use egui::epaint::StrokeKind;

use crate::app::MyApp;
use crate::canvas::CurrentTool;
use crate::canvas::RenderState;
use crate::file_io::PendingFileAction;
use crate::tools::bucket_fill::draw_bucket_fill;
use crate::tools::circle_brush::draw_circle;
use crate::tools::circle_brush::draw_circle_line;
use crate::tools::custom_brush::draw_custom_brush_line;
use crate::tools::square_brush::draw_square;
use crate::tools::square_brush::draw_square_line;
use crate::tool_configuration::StampTintMode;
use crate::tools::stamp_brush::draw_stamp_line;
use crate::undo::UndoRecord;

const PREVIEW_FILL_ALPHA_FACTOR: f32 = 0.2;
const PREVIEW_STROKE_WIDTH: f32 = 1.0;
const ACTIVE_DURATION_MILLISECONDS: u64 = 550;
const CANVAS_BORDER_WIDTH: f32 = 2.0;
const CANVAS_BORDER_COLOR: Color32 = Color32::from_rgb(128, 0, 128);

impl MyApp {
    /// Render the central canvas panel.
    ///
    /// Only renders if a texture exists (wgpu or fallback). Delegates
    /// interaction handling to `handle_canvas_interaction`.
    /// Render the central canvas panel where brush strokes and interaction occur.
    ///
    /// No-op when neither a GPU texture nor an egui-managed texture is available.
    ///
    /// # Parameters
    ///
    /// * `ui` — The egui UI handle.
    pub fn show_central_panel(&mut self, ui: &mut egui::Ui) {
        if self.gpu_texture.is_some() || self.document.canvas.rendered_layers.is_some() {
            self.handle_canvas_interaction(ui);
        }
    }

    /// Process mouse interaction on the canvas texture.
    ///
    /// Handles brush preview rendering, context menu (Import/Export As/Save As),
    /// drawing strokes with the current tool on drag, and bucket fill on click.
    /// Updates the undo history and marks the document as dirty.
    fn handle_canvas_interaction(&mut self, ui: &mut egui::Ui) {
        let (texture_id, canvas_pixel_size) = if let Some(gpu) = &self.gpu_texture {
            (
                gpu.texture_id,
                egui::vec2(
                    self.document.canvas.width as f32,
                    self.document.canvas.height as f32,
                ),
            )
        } else if let Some(tex) = &self.document.canvas.rendered_layers {
            (tex.id(), tex.size_vec2())
        } else {
            return;
        };

        let available = ui.available_size();
        let scale = (available.x / canvas_pixel_size.x).min(available.y / canvas_pixel_size.y);
        let draw_size = canvas_pixel_size * scale;

        let response = ui
            .add(
                egui::Image::new((texture_id, canvas_pixel_size))
                    .fit_to_exact_size(draw_size)
                    .sense(egui::Sense::click_and_drag()),
            )
            .on_hover_cursor(egui::CursorIcon::Crosshair);

        // Draw a dashed purple border around the canvas.
        for dash in egui::Shape::dashed_line(
            &[
                response.rect.left_top(),
                response.rect.right_top(),
                response.rect.right_bottom(),
                response.rect.left_bottom(),
                response.rect.left_top(),
            ],
            egui::Stroke::new(CANVAS_BORDER_WIDTH, CANVAS_BORDER_COLOR),
            6.0,
            4.0,
        ) {
            ui.painter().add(dash);
        }

        response.context_menu(|ui| {
            if ui.button("Import").clicked() {
                self.file_io.pending_file_action = Some(PendingFileAction::Import);
                ui.ctx().request_repaint();
                ui.close();
            }

            ui.menu_button("Export As", |ui| {
                for (format_index, &(label, _)) in crate::app::EXPORT_FORMATS.iter().enumerate() {
                    if ui.button(label).clicked() {
                        self.file_io
                            .queue_file_action(PendingFileAction::Export(format_index));
                        ui.ctx().request_repaint();
                        ui.close();
                    }
                }
            });

            ui.separator();

            if ui.button("Save As").clicked() {
                self.file_io.queue_file_action(PendingFileAction::Save);
                self.document.savefile_path.clear();
                ui.ctx().request_repaint();
                ui.close();
            }

            if self.tool_configuration.current_tool == CurrentTool::Stamp {
                ui.separator();
                if ui.button("Replace Stamp Image...").clicked() {
                    self.file_io.queue_file_action(PendingFileAction::LoadStamp);
                    ui.ctx().request_repaint();
                    ui.close();
                }
            }
            if self.tool_configuration.current_tool == CurrentTool::CustomBrush {
                ui.separator();
                if ui.button("Replace Brush...").clicked() {
                    self.file_io.queue_file_action(PendingFileAction::LoadBrush);
                    ui.ctx().request_repaint();
                    ui.close();
                }
            }
        });

        if self.tool_configuration.show_brush_preview
            && let Some(hover_pos) = response.hover_pos()
        {
            let local_position = hover_pos - response.rect.min;
            let uv = egui::vec2(
                local_position.x / response.rect.width(),
                local_position.y / response.rect.height(),
            );

            let pixel_x = (uv.x * (self.document.canvas.width as f32)).floor() as u32;
            let pixel_y = (uv.y * (self.document.canvas.height as f32)).floor() as u32;

            match self.tool_configuration.current_tool {
                CurrentTool::Circle | CurrentTool::CircleEraser => {
                    let center_screen_x = response.rect.min.x
                        + (pixel_x as f32) * (draw_size.x / (self.document.canvas.width as f32));
                    let center_screen_y = response.rect.min.y
                        + (pixel_y as f32) * (draw_size.y / (self.document.canvas.height as f32));
                    let screen_radius = (self.tool_configuration.radius as f32)
                        * (draw_size.x / (self.document.canvas.width as f32));

                    ui.painter().circle_stroke(
                        Pos2::new(center_screen_x, center_screen_y),
                        screen_radius,
                        egui::Stroke::new(
                            PREVIEW_STROKE_WIDTH,
                            self.tool_configuration.current_color,
                        ),
                    );
                }
                CurrentTool::Square | CurrentTool::SquareEraser => {
                    let half_radius = self.tool_configuration.radius;

                    let preview_start_x = pixel_x.saturating_sub(half_radius) as f32;
                    let preview_end_x =
                        ((pixel_x + half_radius).min(self.document.canvas.width - 1) as f32) + 1.0;
                    let preview_start_y = pixel_y.saturating_sub(half_radius) as f32;
                    let preview_end_y =
                        ((pixel_y + half_radius).min(self.document.canvas.height - 1) as f32) + 1.0;

                    let screen_x = response.rect.min.x
                        + preview_start_x * (draw_size.x / (self.document.canvas.width as f32));
                    let screen_y = response.rect.min.y
                        + preview_start_y * (draw_size.y / (self.document.canvas.height as f32));
                    let screen_w = (preview_end_x - preview_start_x)
                        * (draw_size.x / (self.document.canvas.width as f32));
                    let screen_h = (preview_end_y - preview_start_y)
                        * (draw_size.y / (self.document.canvas.height as f32));

                    let preview_rect = Rect::from_min_size(
                        Pos2::new(screen_x, screen_y),
                        egui::vec2(screen_w, screen_h),
                    );

                    let brush_alpha = self.tool_configuration.current_color.a();
                    let fill_color = if brush_alpha == 0 {
                        Color32::TRANSPARENT
                    } else {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let preview_alpha =
                            ((brush_alpha as f32) * PREVIEW_FILL_ALPHA_FACTOR) as u8;
                        Color32::from_rgba_premultiplied(
                            ((self.tool_configuration.current_color.r() as u32
                                * preview_alpha as u32)
                                / brush_alpha as u32) as u8,
                            ((self.tool_configuration.current_color.g() as u32
                                * preview_alpha as u32)
                                / brush_alpha as u32) as u8,
                            ((self.tool_configuration.current_color.b() as u32
                                * preview_alpha as u32)
                                / brush_alpha as u32) as u8,
                            preview_alpha,
                        )
                    };
                    ui.painter().rect_filled(preview_rect, 0.0, fill_color);

                    ui.painter().rect_stroke(
                        preview_rect,
                        0.0,
                        egui::Stroke::new(
                            PREVIEW_STROKE_WIDTH,
                            self.tool_configuration.current_color,
                        ),
                        StrokeKind::Middle,
                    );
                }
                CurrentTool::BucketFill => {}
                CurrentTool::Stamp => {
                    if let Some(entry) = self.stamp_library.selected() {
                        stamp_tip_preview(
                            ui,
                            &response,
                            pixel_x,
                            pixel_y,
                            draw_size,
                            entry.texture_id(),
                            entry.width,
                            entry.height,
                            self.tool_configuration.radius,
                            self.document.canvas.width,
                            self.document.canvas.height,
                            self.tool_configuration.current_color,
                        );
                    }
                }
                CurrentTool::CustomBrush => {
                    if let Some(entry) = self.brush_library.selected() {
                        stamp_tip_preview(
                            ui,
                            &response,
                            pixel_x,
                            pixel_y,
                            draw_size,
                            entry.texture_id(),
                            entry.width,
                            entry.height,
                            self.tool_configuration.radius,
                            self.document.canvas.width,
                            self.document.canvas.height,
                            self.tool_configuration.current_color,
                        );
                    }
                }
            }
        }

        if response.hovered() {
            self.ui.pending_layer_for_deletion = None;
            self.ui.render_state =
                RenderState::ActiveWake(Duration::from_millis(ACTIVE_DURATION_MILLISECONDS));
        }

        if response.clicked()
            && self.tool_configuration.current_tool == CurrentTool::Stamp
            && self.stamp_library.is_empty()
        {
            self.file_io.queue_file_action(PendingFileAction::LoadStamp);
            ui.ctx().request_repaint();
        }
        if response.clicked()
            && self.tool_configuration.current_tool == CurrentTool::CustomBrush
            && self.brush_library.is_empty()
        {
            self.file_io.queue_file_action(PendingFileAction::LoadBrush);
            ui.ctx().request_repaint();
        }

        if response.clicked() && self.tool_configuration.current_tool == CurrentTool::BucketFill {
            if let Some(position) = response.interact_pointer_pos() {
                let local_position = position - response.rect.min;
                let uv = egui::vec2(
                    local_position.x / response.rect.width(),
                    local_position.y / response.rect.height(),
                );

                let pixel_x = (uv.x * (self.document.canvas.width as f32)).floor() as u32;
                let pixel_y = (uv.y * (self.document.canvas.height as f32)).floor() as u32;

                Arc::make_mut(&mut self.document.canvas).render_next_frame = true;
                let stroke = draw_bucket_fill(
                    pixel_x,
                    pixel_y,
                    Arc::make_mut(&mut self.document.canvas),
                    self.tool_configuration.current_color,
                    self.document.current_layer,
                    self.tool_configuration.alpha_overlay,
                );
                self.undo.push_undo(stroke);
                self.document.dirty_since_last_autosave = true;
            }
        }

        if response.dragged() {
            if let Some(position) = response.interact_pointer_pos() {
                let local_position = position - response.rect.min;
                let uv = egui::vec2(
                    local_position.x / response.rect.width(),
                    local_position.y / response.rect.height(),
                );

                let pixel_x = (uv.x * (self.document.canvas.width as f32)).floor() as u32;
                let pixel_y = (uv.y * (self.document.canvas.height as f32)).floor() as u32;

                if self.tool_configuration.current_tool != CurrentTool::BucketFill {
                    Arc::make_mut(&mut self.document.canvas).render_next_frame = true;
                    if let Some(stroke) = self.apply_stroke(pixel_x, pixel_y) {
                        self.document.dirty_since_last_autosave = true;
                        if self.ui.previous_cursor_position.is_none() {
                            let UndoRecord::Run {
                                layer_index,
                                color_after,
                                runs,
                                is_alpha_overlay,
                            } = stroke;
                            self.undo.init_drag_accumulator(
                                layer_index,
                                self.document.canvas.width,
                                color_after,
                                is_alpha_overlay,
                            );
                            self.undo.extend_drag_accumulator(runs);
                        } else {
                            let UndoRecord::Run { runs, .. } = stroke;
                            self.undo.extend_drag_accumulator(runs);
                        }
                    }
                }

                self.ui.previous_tool = Some(self.tool_configuration.current_tool);
                self.ui.previous_cursor_position = Some((pixel_x, pixel_y));
            }
        } else {
            self.undo.finalize_drag_accumulator();
            self.ui.previous_tool = None;
            self.ui.previous_cursor_position = None;
        }
    }

    /// Apply the current drawing tool at the given pixel coordinates.
    ///
    /// Returns `Some(UndoRecord)` if a stroke was applied, or `None` for tools
    /// that are handled via click (bucket fill).
    ///
    /// Erasers use `Color32::TRANSPARENT` with alpha overlay disabled.
    /// Square and Circle tools handle first-frame (stamp) vs subsequent-frame (line) logic.
    fn apply_stroke(&mut self, pixel_x: u32, pixel_y: u32) -> Option<UndoRecord> {
        let is_eraser = matches!(
            self.tool_configuration.current_tool,
            CurrentTool::SquareEraser | CurrentTool::CircleEraser
        );
        let color = if is_eraser {
            Color32::TRANSPARENT
        } else {
            self.tool_configuration.current_color
        };
        let alpha_overlay = self.tool_configuration.alpha_overlay && !is_eraser;

        match self.tool_configuration.current_tool {
            CurrentTool::BucketFill => None,

            CurrentTool::Square | CurrentTool::SquareEraser => {
                let first_frame = self.ui.previous_cursor_position.is_none();
                let previous_position = self.ui.previous_cursor_position;

                if first_frame {
                    if alpha_overlay {
                        self.undo.advance_drag_stamp();
                        let stamp = self.undo.next_stamp();
                        Some(draw_square_line(
                            pixel_x,
                            pixel_y,
                            pixel_x,
                            pixel_y,
                            self.tool_configuration.radius,
                            Arc::make_mut(&mut self.document.canvas),
                            color,
                            self.document.current_layer,
                            &mut self.undo.visited,
                            stamp,
                            true,
                            &mut self.undo.drag_processed,
                            self.undo.drag_stamp_value,
                        ))
                    } else {
                        let half_radius = self.tool_configuration.radius;
                        let start_x = pixel_x.saturating_sub(half_radius);
                        let end_x = (pixel_x + half_radius).min(self.document.canvas.width - 1);
                        let start_y = pixel_y.saturating_sub(half_radius);
                        let end_y = (pixel_y + half_radius).min(self.document.canvas.height - 1);
                        Some(draw_square(
                            start_x,
                            start_y,
                            end_x,
                            end_y,
                            Arc::make_mut(&mut self.document.canvas),
                            color,
                            self.document.current_layer,
                            false,
                        ))
                    }
                } else if let Some((previous_x, previous_y)) = previous_position {
                    let stamp = self.undo.next_stamp();
                    Some(draw_square_line(
                        previous_x,
                        previous_y,
                        pixel_x,
                        pixel_y,
                        self.tool_configuration.radius,
                        Arc::make_mut(&mut self.document.canvas),
                        color,
                        self.document.current_layer,
                        &mut self.undo.visited,
                        stamp,
                        alpha_overlay,
                        &mut self.undo.drag_processed,
                        self.undo.drag_stamp_value,
                    ))
                } else {
                    None
                }
            }

            CurrentTool::Circle | CurrentTool::CircleEraser => {
                let first_frame = self.ui.previous_cursor_position.is_none();
                let previous_position = self.ui.previous_cursor_position;

                if first_frame {
                    if alpha_overlay {
                        self.undo.advance_drag_stamp();
                        let stamp = self.undo.next_stamp();
                        Some(draw_circle_line(
                            pixel_x,
                            pixel_y,
                            pixel_x,
                            pixel_y,
                            self.tool_configuration.radius,
                            Arc::make_mut(&mut self.document.canvas),
                            color,
                            self.document.current_layer,
                            &mut self.undo.visited,
                            stamp,
                            true,
                            &mut self.undo.drag_processed,
                            self.undo.drag_stamp_value,
                        ))
                    } else {
                        Some(draw_circle(
                            pixel_x,
                            pixel_y,
                            self.tool_configuration.radius,
                            Arc::make_mut(&mut self.document.canvas),
                            color,
                            self.document.current_layer,
                            false,
                        ))
                    }
                } else if let Some((previous_x, previous_y)) = previous_position {
                    let stamp = self.undo.next_stamp();
                    Some(draw_circle_line(
                        previous_x,
                        previous_y,
                        pixel_x,
                        pixel_y,
                        self.tool_configuration.radius,
                        Arc::make_mut(&mut self.document.canvas),
                        color,
                        self.document.current_layer,
                        &mut self.undo.visited,
                        stamp,
                        alpha_overlay,
                        &mut self.undo.drag_processed,
                        self.undo.drag_stamp_value,
                    ))
                } else {
                    None
                }
            }

            CurrentTool::Stamp => {
                let first_frame = self.ui.previous_cursor_position.is_none();
                let previous_position = self.ui.previous_cursor_position;

                if first_frame && alpha_overlay {
                    self.undo.advance_drag_stamp();
                }

                let (start_x, start_y) = if first_frame {
                    (pixel_x, pixel_y)
                } else if let Some((px, py)) = previous_position {
                    (px, py)
                } else {
                    (pixel_x, pixel_y)
                };

                let stamp = self.undo.next_stamp();

                self.stamp_library.selected().map(|entry| {
                    draw_stamp_line(
                        start_x,
                        start_y,
                        pixel_x,
                        pixel_y,
                        &entry.pixels,
                        entry.width,
                        entry.height,
                        self.tool_configuration.radius,
                        Arc::make_mut(&mut self.document.canvas),
                        color,
                        self.document.current_layer,
                        &mut self.undo.visited,
                        stamp,
                        alpha_overlay,
                        self.tool_configuration.stamp_tint_mode
                            == StampTintMode::Tinted,
                        self.tool_configuration.stamp_sampling,
                        &mut self.undo.drag_processed,
                        self.undo.drag_stamp_value,
                    )
                })
            }

            CurrentTool::CustomBrush => {
                let first_frame = self.ui.previous_cursor_position.is_none();
                let previous_position = self.ui.previous_cursor_position;

                if first_frame && alpha_overlay {
                    self.undo.advance_drag_stamp();
                }

                let (start_x, start_y) = if first_frame {
                    (pixel_x, pixel_y)
                } else if let Some((px, py)) = previous_position {
                    (px, py)
                } else {
                    (pixel_x, pixel_y)
                };

                let stamp = self.undo.next_stamp();

                self.brush_library.selected().map(|entry| {
                    draw_custom_brush_line(
                        start_x,
                        start_y,
                        pixel_x,
                        pixel_y,
                        &entry.pixels,
                        entry.width,
                        entry.height,
                        self.tool_configuration.radius,
                        entry.spacing,
                        Arc::make_mut(&mut self.document.canvas),
                        color,
                        self.document.current_layer,
                        &mut self.undo.visited,
                        stamp,
                        alpha_overlay,
                        self.tool_configuration.brush_tint_mode
                            == StampTintMode::Tinted,
                        self.tool_configuration.brush_sampling,
                        &mut self.undo.drag_processed,
                        self.undo.drag_stamp_value,
                    )
                })
            }
        }
    }
}

/// Draw a stamp/brush-tip preview: renders the tip image at the cursor
/// position scaled to `radius` width, with a border in `border_color`.
fn stamp_tip_preview(
    ui: &egui::Ui,
    response: &egui::Response,
    pixel_x: u32,
    pixel_y: u32,
    draw_size: egui::Vec2,
    tex_id: Option<egui::TextureId>,
    tip_width: u32,
    tip_height: u32,
    radius: u32,
    canvas_width: u32,
    canvas_height: u32,
    border_color: Color32,
) {
    let output_w = radius.max(1);
    let output_h = ((tip_height as f64 * output_w as f64 / tip_width as f64).round() as u32).max(1);
    let half_w = output_w / 2;
    let half_h = output_h / 2;

    let preview_start_x = (pixel_x.saturating_sub(half_w)).min(canvas_width - 1) as f32;
    let preview_end_x = ((pixel_x + half_w).min(canvas_width - 1) as f32) + 1.0;
    let preview_start_y = (pixel_y.saturating_sub(half_h)).min(canvas_height - 1) as f32;
    let preview_end_y = ((pixel_y + half_h).min(canvas_height - 1) as f32) + 1.0;

    let screen_x = response.rect.min.x + preview_start_x * (draw_size.x / (canvas_width as f32));
    let screen_y = response.rect.min.y + preview_start_y * (draw_size.y / (canvas_height as f32));
    let screen_w = (preview_end_x - preview_start_x) * (draw_size.x / (canvas_width as f32));
    let screen_h = (preview_end_y - preview_start_y) * (draw_size.y / (canvas_height as f32));

    let preview_rect = Rect::from_min_size(
        Pos2::new(screen_x, screen_y),
        egui::vec2(screen_w, screen_h),
    );

    if let Some(tid) = tex_id {
        ui.painter().image(
            tid,
            preview_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    ui.painter().rect_stroke(
        preview_rect,
        0.0,
        egui::Stroke::new(PREVIEW_STROKE_WIDTH, border_color),
        StrokeKind::Middle,
    );
}
