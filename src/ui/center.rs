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
use crate::brush_params::BrushStrokeParams;
use crate::canvas::CurrentTool;
use crate::canvas::RenderState;
use crate::file_io::PendingFileAction;
use crate::pixel;
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
const ZOOM_SENSITIVITY: f32 = 0.003;
const MIN_ZOOM: f32 = 0.05;
const MAX_ZOOM: f32 = 20.0;
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
        let base_size = canvas_pixel_size * scale;
        let draw_size = base_size * self.ui.zoom;

        let canvas_pos = egui::pos2(
            (available.x - draw_size.x) / 2.0 + self.ui.pan_offset.x,
            (available.y - draw_size.y) / 2.0 + self.ui.pan_offset.y,
        );
        let canvas_rect = Rect::from_min_size(canvas_pos, draw_size);

        let cursor = if self.tool_configuration.current_tool == CurrentTool::Pan {
            egui::CursorIcon::Grab
        } else {
            egui::CursorIcon::Crosshair
        };

        let response = ui
            .put(
                canvas_rect,
                egui::Image::new((texture_id, canvas_pixel_size))
                    .fit_to_exact_size(draw_size)
                    .sense(egui::Sense::click_and_drag()),
            )
            .on_hover_cursor(cursor);

        // Draw a dashed purple border around the canvas.
        for dash in egui::Shape::dashed_line(
            &[
                canvas_rect.left_top(),
                canvas_rect.right_top(),
                canvas_rect.right_bottom(),
                canvas_rect.left_bottom(),
                canvas_rect.left_top(),
            ],
            egui::Stroke::new(CANVAS_BORDER_WIDTH, CANVAS_BORDER_COLOR),
            6.0,
            4.0,
        ) {
            ui.painter().add(dash);
        }

        // Draw grid overlay if enabled.
        if self.tool_configuration.show_grid {
            let grid = self.tool_configuration.grid_size.max(1);
            let cw = self.document.canvas.width;
            let ch = self.document.canvas.height;
            let sx = draw_size.x / cw as f32;
            let sy = draw_size.y / ch as f32;
            let grid_color = egui::Color32::from_gray(128);
            let grid_stroke = egui::Stroke::new(1.0, grid_color);

            // Vertical lines
            let mut x = grid as f32;
            while x < cw as f32 {
                let screen_x = canvas_rect.min.x + x * sx;
                ui.painter().line_segment(
                    [egui::pos2(screen_x, canvas_rect.top()), egui::pos2(screen_x, canvas_rect.bottom())],
                    grid_stroke,
                );
                x += grid as f32;
            }
            // Horizontal lines
            let mut y = grid as f32;
            while y < ch as f32 {
                let screen_y = canvas_rect.min.y + y * sy;
                ui.painter().line_segment(
                    [egui::pos2(canvas_rect.left(), screen_y), egui::pos2(canvas_rect.right(), screen_y)],
                    grid_stroke,
                );
                y += grid as f32;
            }
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
                        self.ui.last_export_format = format_index;
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

        // Zoom via scroll/pinch while hovering the canvas.
        if response.hovered() {
            let pinch = ui.input(|i| i.zoom_delta());
            let scroll = ui.input(|i| i.smooth_scroll_delta());
            let zoom_input = if pinch != 1.0 {
                pinch
            } else if scroll.y != 0.0 {
                1.0 + scroll.y * ZOOM_SENSITIVITY
            } else {
                1.0
            };
            if zoom_input != 1.0 {
                let old_zoom = self.ui.zoom;
                self.ui.zoom = (old_zoom * zoom_input).clamp(MIN_ZOOM, MAX_ZOOM);
                if self.ui.zoom != old_zoom {
                    if let Some(hover) = response.hover_pos() {
                        let old_draw = base_size * old_zoom;
                        let frac_x = (hover.x - canvas_pos.x) / old_draw.x;
                        let frac_y = (hover.y - canvas_pos.y) / old_draw.y;
                        let new_draw = base_size * self.ui.zoom;
                        let nx = (available.x - new_draw.x) / 2.0;
                        let ny = (available.y - new_draw.y) / 2.0;
                        self.ui.pan_offset.x = hover.x - nx - frac_x * new_draw.x;
                        self.ui.pan_offset.y = hover.y - ny - frac_y * new_draw.y;
                    }
                    ui.ctx().request_repaint();
                }
            }
        }

        // Reset zoom on double-click.
        if response.double_clicked() {
            self.ui.zoom = 1.0;
            ui.ctx().request_repaint();
        }

        if self.tool_configuration.show_brush_preview
            && let Some(hover_pos) = response.hover_pos()
        {
            let local_position = hover_pos - response.rect.min;
            let uv = egui::vec2(
                local_position.x / response.rect.width(),
                local_position.y / response.rect.height(),
            );

                let pixel_x = (uv.x * (self.document.canvas.width as f32))
                    .floor()
                    .min((self.document.canvas.width - 1) as f32) as u32;
                let pixel_y = (uv.y * (self.document.canvas.height as f32))
                    .floor()
                    .min((self.document.canvas.height - 1) as f32) as u32;

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
                        // SAFETY: `brush_alpha ≤ 255`, `PREVIEW_FILL_ALPHA_FACTOR ≈ 0.5`,
                        // so float result ≤ 127.5 → truncation to u8 is safe;
                        // all intermediates are non-negative → no sign loss.
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
                CurrentTool::Eyedropper => {}
                CurrentTool::Pan => {}
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

                let pixel_x = (uv.x * (self.document.canvas.width as f32))
                    .floor()
                    .min((self.document.canvas.width - 1) as f32) as u32;
                let pixel_y = (uv.y * (self.document.canvas.height as f32))
                    .floor()
                    .min((self.document.canvas.height - 1) as f32) as u32;

                let canvas = Arc::make_mut(&mut self.document.canvas);
                canvas.dirty_rect.request_full_blend();
                let stroke = draw_bucket_fill(
                    pixel_x,
                    pixel_y,
                    canvas,
                    self.tool_configuration.current_color,
                    self.document.current_layer,
                    self.tool_configuration.alpha_overlay,
                );
                self.undo.push_undo(stroke);
                self.document.dirty_since_last_autosave = true;
            }
        }

        if response.clicked()
            && self.tool_configuration.current_tool == CurrentTool::Eyedropper
        {
            if let Some(position) = response.interact_pointer_pos() {
                let local_position = position - response.rect.min;
                let uv = egui::vec2(
                    local_position.x / response.rect.width(),
                    local_position.y / response.rect.height(),
                );
                let pixel_x = (uv.x * (self.document.canvas.width as f32))
                    .floor()
                    .min((self.document.canvas.width - 1) as f32) as u32;
                let pixel_y = (uv.y * (self.document.canvas.height as f32))
                    .floor()
                    .min((self.document.canvas.height - 1) as f32) as u32;
                let w = self.document.canvas.width;
                let index = ((pixel_y * w + pixel_x) as usize) * 4;
                let rgba = &self.document.canvas.output_rgba;
                if index + 3 < rgba.len() {
                    let premul = Color32::from_rgba_premultiplied(
                        rgba[index],
                        rgba[index + 1],
                        rgba[index + 2],
                        rgba[index + 3],
                    );
                    self.tool_configuration.current_color = pixel::unpremultiply(premul);
                }
            }
        }

        if response.dragged() && self.tool_configuration.current_tool == CurrentTool::Pan {
            self.ui.pan_offset += response.drag_delta();
            ui.ctx().request_repaint();
        }

        if response.dragged() {
            if let Some(position) = response.interact_pointer_pos() {
                let local_position = position - response.rect.min;
                let uv = egui::vec2(
                    local_position.x / response.rect.width(),
                    local_position.y / response.rect.height(),
                );

                let pixel_x = (uv.x * (self.document.canvas.width as f32))
                    .floor()
                    .min((self.document.canvas.width - 1) as f32) as u32;
                let pixel_y = (uv.y * (self.document.canvas.height as f32))
                    .floor()
                    .min((self.document.canvas.height - 1) as f32) as u32;

                if !matches!(
                    self.tool_configuration.current_tool,
                    CurrentTool::BucketFill | CurrentTool::Eyedropper | CurrentTool::Pan
                ) {
                    if let Some(stroke) = self.apply_stroke(pixel_x, pixel_y) {
                        self.document.dirty_since_last_autosave = true;
                        if self.ui.previous_cursor_position.is_none() {
                            if let UndoRecord::Run {
                                layer_index,
                                color_after,
                                runs,
                                is_alpha_overlay,
                            } = stroke
                            {
                                self.undo.init_drag_accumulator(
                                    layer_index,
                                    self.document.canvas.width,
                                    color_after,
                                    is_alpha_overlay,
                                );
                                self.undo.extend_drag_accumulator(runs);
                            }
                        } else if let UndoRecord::Run { runs, .. } = stroke {
                            self.undo.extend_drag_accumulator(runs);
                        }
                    }
                }

                self.ui.previous_cursor_position = Some((pixel_x, pixel_y));
            }
        } else {
            self.undo.finalize_drag_accumulator();
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

        let canvas = Arc::make_mut(&mut self.document.canvas);
        canvas.dirty_rect.request_full_blend();

        match self.tool_configuration.current_tool {
            CurrentTool::BucketFill => None,
            CurrentTool::Eyedropper => None,
            CurrentTool::Pan => None,

            CurrentTool::Square | CurrentTool::SquareEraser => {
                let first_frame = self.ui.previous_cursor_position.is_none();
                let previous_position = self.ui.previous_cursor_position;

                if first_frame {
                    if alpha_overlay {
                        self.undo.advance_drag_stamp();
                        let stamp = self.undo.next_stamp();
                        let (visited, drag_processed, ds_val) = self.undo.scratch_buffers();
                        Some(draw_square_line(
                            BrushStrokeParams {
                                start_x: pixel_x,
                                start_y: pixel_y,
                                end_x: pixel_x,
                                end_y: pixel_y,
                                canvas,
                                color,
                                layer: self.document.current_layer,
                                visited,
                                stamp,
                                alpha_overlay: true,
                                drag_processed,
                                drag_stamp_value: ds_val,
                            },
                            self.tool_configuration.radius,
                        ))
                    } else {
                        let half_radius = self.tool_configuration.radius;
                        let start_x = pixel_x.saturating_sub(half_radius);
                        let end_x = (pixel_x + half_radius + 1).min(canvas.width);
                        let start_y = pixel_y.saturating_sub(half_radius);
                        let end_y = (pixel_y + half_radius + 1).min(canvas.height);
                        Some(draw_square(
                            start_x,
                            start_y,
                            end_x,
                            end_y,
                            canvas,
                            color,
                            self.document.current_layer,
                            false,
                        ))
                    }
                } else if let Some((previous_x, previous_y)) = previous_position {
                    let stamp = self.undo.next_stamp();
                    let (visited, drag_processed, ds_val) = self.undo.scratch_buffers();
                    Some(draw_square_line(
                        BrushStrokeParams {
                            start_x: previous_x,
                            start_y: previous_y,
                            end_x: pixel_x,
                            end_y: pixel_y,
                            canvas,
                            color,
                            layer: self.document.current_layer,
                            visited,
                            stamp,
                            alpha_overlay,
                            drag_processed,
                            drag_stamp_value: ds_val,
                        },
                        self.tool_configuration.radius,
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
                        let (visited, drag_processed, ds_val) = self.undo.scratch_buffers();
                        Some(draw_circle_line(
                            BrushStrokeParams {
                                start_x: pixel_x,
                                start_y: pixel_y,
                                end_x: pixel_x,
                                end_y: pixel_y,
                                canvas,
                                color,
                                layer: self.document.current_layer,
                                visited,
                                stamp,
                                alpha_overlay: true,
                                drag_processed,
                                drag_stamp_value: ds_val,
                            },
                            self.tool_configuration.radius,
                        ))
                    } else {
                        Some(draw_circle(
                            pixel_x,
                            pixel_y,
                            self.tool_configuration.radius,
                            canvas,
                            color,
                            self.document.current_layer,
                            false,
                        ))
                    }
                } else if let Some((previous_x, previous_y)) = previous_position {
                    let stamp = self.undo.next_stamp();
                    let (visited, drag_processed, ds_val) = self.undo.scratch_buffers();
                    Some(draw_circle_line(
                        BrushStrokeParams {
                            start_x: previous_x,
                            start_y: previous_y,
                            end_x: pixel_x,
                            end_y: pixel_y,
                            canvas,
                            color,
                            layer: self.document.current_layer,
                            visited,
                            stamp,
                            alpha_overlay,
                            drag_processed,
                            drag_stamp_value: ds_val,
                        },
                        self.tool_configuration.radius,
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
                    let (visited, drag_processed, ds_val) = self.undo.scratch_buffers();
                    draw_stamp_line(
                        BrushStrokeParams {
                            start_x,
                            start_y,
                            end_x: pixel_x,
                            end_y: pixel_y,
                            canvas,
                            color,
                            layer: self.document.current_layer,
                            visited,
                            stamp,
                            alpha_overlay,
                            drag_processed,
                            drag_stamp_value: ds_val,
                        },
                        &entry.pixels,
                        entry.width,
                        entry.height,
                        self.tool_configuration.radius,
                        self.tool_configuration.stamp_tint_mode
                            == StampTintMode::Tinted,
                        self.tool_configuration.stamp_sampling,
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
                    let (visited, drag_processed, ds_val) = self.undo.scratch_buffers();
                    draw_custom_brush_line(
                        BrushStrokeParams {
                            start_x,
                            start_y,
                            end_x: pixel_x,
                            end_y: pixel_y,
                            canvas,
                            color,
                            layer: self.document.current_layer,
                            visited,
                            stamp,
                            alpha_overlay,
                            drag_processed,
                            drag_stamp_value: ds_val,
                        },
                        &entry.pixels,
                        entry.width,
                        entry.height,
                        self.tool_configuration.radius,
                        entry.spacing,
                        self.tool_configuration.brush_tint_mode
                            == StampTintMode::Tinted,
                        self.tool_configuration.brush_sampling,
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
