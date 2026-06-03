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
use crate::app::ProgressState;
use crate::brush_params::BrushStrokeParams;
use crate::canvas::CurrentTool;
use crate::canvas::RenderState;
use crate::file_io::PendingFileAction;
use crate::pixel;
use crate::tool_configuration::StampTintMode;
use crate::tools::bucket_fill::draw_bucket_fill;
use crate::tools::circle_brush::draw_circle;
use crate::tools::circle_brush::draw_circle_line;
use crate::tools::custom_brush::draw_custom_brush_line;
use crate::tools::square_brush::draw_square;
use crate::tools::square_brush::draw_square_line;
use crate::tools::stamp_brush::draw_stamp_line;
use crate::undo::UndoRecord;

/// Alpha multiplier applied to the brush preview fill color.
const PREVIEW_FILL_ALPHA_FACTOR: f32 = 0.2;
const PREVIEW_STROKE_WIDTH: f32 = 1.0;
const ZOOM_SENSITIVITY: f32 = 0.003;
const MIN_ZOOM: f32 = 0.05;
const MAX_ZOOM: f32 = 20.0;
const ACTIVE_DURATION_MILLISECONDS: u64 = 550;
const CANVAS_BORDER_WIDTH: f32 = 2.0;
const CANVAS_BORDER_COLOR: Color32 = Color32::from_rgb(128, 0, 128);

/// Convert UV coordinates (0..1) to canvas pixel coordinates, clamped to
/// the valid pixel range.
fn uv_to_pixel(uv: egui::Vec2, canvas_width: u32, canvas_height: u32) -> (u32, u32) {
    let pixel_x = (uv.x * (canvas_width as f32))
        .floor()
        .min((canvas_width - 1) as f32) as u32;
    let pixel_y = (uv.y * (canvas_height as f32))
        .floor()
        .min((canvas_height - 1) as f32) as u32;
    (pixel_x, pixel_y)
}

/// Compute the pan offset that keeps a cursor point anchored on the canvas
/// when the zoom level changes. This adjusts `pan_offset` so the pixel
/// under the cursor stays in the same screen position.
fn zoom_around_point(
    old_zoom: f32,
    new_zoom: f32,
    hover_pos: egui::Pos2,
    canvas_pos: egui::Pos2,
    base_size: egui::Vec2,
    available: egui::Vec2,
) -> egui::Vec2 {
    let old_draw = base_size * old_zoom;
    let frac_x = (hover_pos.x - canvas_pos.x) / old_draw.x;
    let frac_y = (hover_pos.y - canvas_pos.y) / old_draw.y;
    let new_draw = base_size * new_zoom;
    let nx = (available.x - new_draw.x) / 2.0;
    let ny = (available.y - new_draw.y) / 2.0;
    egui::vec2(
        hover_pos.x - nx - frac_x * new_draw.x,
        hover_pos.y - ny - frac_y * new_draw.y,
    )
}

/// Compute the canvas draw size and screen rectangle from layout parameters.
///
/// Centers the canvas within the available area, applies zoom and pan offset.
///
/// # Returns
///
/// A tuple of `(draw_size, canvas_rect)` where `draw_size` is the scaled
/// canvas size and `canvas_rect` is the screen-space rectangle.
fn compute_canvas_rect(
    available: egui::Vec2,
    base_size: egui::Vec2,
    zoom: f32,
    pan_offset: egui::Vec2,
) -> (egui::Vec2, Rect) {
    let draw_size = base_size * zoom;
    let canvas_pos = egui::pos2(
        (available.x - draw_size.x) / 2.0 + pan_offset.x,
        (available.y - draw_size.y) / 2.0 + pan_offset.y,
    );
    (draw_size, Rect::from_min_size(canvas_pos, draw_size))
}

/// Draw a circle brush preview at the cursor position.
///
/// Renders a circle outline at the stabilized cursor position with the
/// current brush radius and color.
fn draw_circle_preview(
    ui: &egui::Ui,
    response: &egui::Response,
    pixel_x: u32,
    pixel_y: u32,
    draw_size: egui::Vec2,
    radius: u32,
    color: Color32,
    canvas_width: u32,
    canvas_height: u32,
) {
    let center_screen_x =
        response.rect.min.x + (pixel_x as f32) * (draw_size.x / (canvas_width as f32));
    let center_screen_y =
        response.rect.min.y + (pixel_y as f32) * (draw_size.y / (canvas_height as f32));
    let screen_radius = (radius as f32) * (draw_size.x / (canvas_width as f32));

    ui.painter().circle_stroke(
        Pos2::new(center_screen_x, center_screen_y),
        screen_radius,
        egui::Stroke::new(PREVIEW_STROKE_WIDTH, color),
    );
}

/// Draw a square brush preview at the cursor position.
///
/// Renders a filled rectangle with the current brush radius and color,
/// using a semi-transparent fill and an opaque border stroke.
fn draw_square_preview(
    ui: &egui::Ui,
    response: &egui::Response,
    pixel_x: u32,
    pixel_y: u32,
    draw_size: egui::Vec2,
    radius: u32,
    color: Color32,
    canvas_width: u32,
    canvas_height: u32,
) {
    let half_radius = radius;

    let preview_start_x = pixel_x.saturating_sub(half_radius) as f32;
    let preview_end_x = ((pixel_x + half_radius).min(canvas_width - 1) as f32) + 1.0;
    let preview_start_y = pixel_y.saturating_sub(half_radius) as f32;
    let preview_end_y = ((pixel_y + half_radius).min(canvas_height - 1) as f32) + 1.0;

    let screen_x = response.rect.min.x + preview_start_x * (draw_size.x / (canvas_width as f32));
    let screen_y = response.rect.min.y + preview_start_y * (draw_size.y / (canvas_height as f32));
    let screen_w = (preview_end_x - preview_start_x) * (draw_size.x / (canvas_width as f32));
    let screen_h = (preview_end_y - preview_start_y) * (draw_size.y / (canvas_height as f32));

    let preview_rect = Rect::from_min_size(
        Pos2::new(screen_x, screen_y),
        egui::vec2(screen_w, screen_h),
    );

    let brush_alpha = color.a();
    let fill_color = if brush_alpha == 0 {
        Color32::TRANSPARENT
    } else {
        // SAFETY: `brush_alpha ≤ 255`, `PREVIEW_FILL_ALPHA_FACTOR ≈ 0.5`,
        // so float result ≤ 127.5 → truncation to u8 is safe;
        // all intermediates are non-negative → no sign loss.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let preview_alpha = ((brush_alpha as f32) * PREVIEW_FILL_ALPHA_FACTOR) as u8;
        Color32::from_rgba_premultiplied(
            ((color.r() as u32 * preview_alpha as u32) / brush_alpha as u32) as u8,
            ((color.g() as u32 * preview_alpha as u32) / brush_alpha as u32) as u8,
            ((color.b() as u32 * preview_alpha as u32) / brush_alpha as u32) as u8,
            preview_alpha,
        )
    };
    ui.painter().rect_filled(preview_rect, 0.0, fill_color);

    ui.painter().rect_stroke(
        preview_rect,
        0.0,
        egui::Stroke::new(PREVIEW_STROKE_WIDTH, color),
        StrokeKind::Middle,
    );
}

impl MyApp {
    /// Render the central canvas panel.
    ///
    /// Only renders if a texture exists (wgpu or fallback). Delegates
    /// interaction handling to `handle_canvas_interaction` and
    /// renders a persistent status bar below the canvas.
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
        self.show_canvas_status_line(ui);
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
        let (draw_size, canvas_rect) =
            compute_canvas_rect(available, base_size, self.ui.zoom, self.ui.pan_offset);

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
            let cache_key = (
                self.tool_configuration.grid_size,
                self.document.canvas.width,
                self.document.canvas.height,
            );
            draw_grid_overlay(
                ui,
                &mut self.ui.grid_cache,
                cache_key,
                canvas_rect,
                draw_size,
            );
        }

        response.context_menu(|ui| {
            if ui.button("Import").clicked() {
                self.dialog_manager.pending_file_action = Some(PendingFileAction::Import);
                ui.ctx().request_repaint();
                ui.close();
            }

            ui.menu_button("Export As", |ui| {
                for (format_index, &(label, _)) in crate::app::EXPORT_FORMATS.iter().enumerate() {
                    if ui.button(label).clicked() {
                        self.ui.last_export_format = format_index;
                        self.dialog_manager
                            .queue_file_action(PendingFileAction::Export(format_index));
                        ui.ctx().request_repaint();
                        ui.close();
                    }
                }
            });

            ui.separator();

            if ui.button("Save As").clicked() {
                self.dialog_manager
                    .queue_file_action(PendingFileAction::Save);
                self.document.savefile_path.clear();
                ui.ctx().request_repaint();
                ui.close();
            }

            if self.tool_configuration.current_tool == CurrentTool::Stamp {
                ui.separator();
                if ui.button("Replace Stamp Image...").clicked() {
                    self.dialog_manager
                        .queue_file_action(PendingFileAction::LoadStamp);
                    ui.ctx().request_repaint();
                    ui.close();
                }
            }
            if self.tool_configuration.current_tool == CurrentTool::CustomBrush {
                ui.separator();
                if ui.button("Replace Brush...").clicked() {
                    self.dialog_manager
                        .queue_file_action(PendingFileAction::LoadBrush);
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
                        self.ui.pan_offset = zoom_around_point(
                            old_zoom,
                            self.ui.zoom,
                            hover,
                            canvas_rect.min,
                            base_size,
                            available,
                        );
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

            let (raw_pixel_x, raw_pixel_y) =
                uv_to_pixel(uv, self.document.canvas.width, self.document.canvas.height);

            let dt = ui.input(|i| i.unstable_dt);
            let (pixel_x, pixel_y) = self.stabilized_pixel(raw_pixel_x, raw_pixel_y, dt);

            // Faint dot at the raw (non-stabilized) cursor position.
            ui.painter()
                .circle_filled(hover_pos, 2.5, Color32::from_gray(80));

            match self.tool_configuration.current_tool {
                CurrentTool::Circle | CurrentTool::CircleEraser => {
                    draw_circle_preview(
                        ui,
                        &response,
                        pixel_x,
                        pixel_y,
                        draw_size,
                        self.tool_configuration.radius,
                        self.tool_configuration.current_color,
                        self.document.canvas.width,
                        self.document.canvas.height,
                    );
                }

                CurrentTool::Square | CurrentTool::SquareEraser => {
                    draw_square_preview(
                        ui,
                        &response,
                        pixel_x,
                        pixel_y,
                        draw_size,
                        self.tool_configuration.radius,
                        self.tool_configuration.current_color,
                        self.document.canvas.width,
                        self.document.canvas.height,
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
            self.dialog_manager
                .queue_file_action(PendingFileAction::LoadStamp);
            ui.ctx().request_repaint();
        }
        if response.clicked()
            && self.tool_configuration.current_tool == CurrentTool::CustomBrush
            && self.brush_library.is_empty()
        {
            self.dialog_manager
                .queue_file_action(PendingFileAction::LoadBrush);
            ui.ctx().request_repaint();
        }

        if response.clicked() && self.tool_configuration.current_tool == CurrentTool::BucketFill {
            if let Some(position) = response.interact_pointer_pos() {
                let local_position = position - response.rect.min;
                let uv = egui::vec2(
                    local_position.x / response.rect.width(),
                    local_position.y / response.rect.height(),
                );

                let (pixel_x, pixel_y) =
                    uv_to_pixel(uv, self.document.canvas.width, self.document.canvas.height);

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

        if response.clicked() && self.tool_configuration.current_tool == CurrentTool::Eyedropper {
            if let Some(position) = response.interact_pointer_pos() {
                let local_position = position - response.rect.min;
                let uv = egui::vec2(
                    local_position.x / response.rect.width(),
                    local_position.y / response.rect.height(),
                );
                let (pixel_x, pixel_y) =
                    uv_to_pixel(uv, self.document.canvas.width, self.document.canvas.height);
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

                let (raw_x, raw_y) =
                    uv_to_pixel(uv, self.document.canvas.width, self.document.canvas.height);

                if !matches!(
                    self.tool_configuration.current_tool,
                    CurrentTool::BucketFill | CurrentTool::Eyedropper | CurrentTool::Pan
                ) {
                    let dt = ui.input(|i| i.unstable_dt);
                    let (stab_x, stab_y) = self.stabilized_pixel(raw_x, raw_y, dt);

                    if let Some(stroke) = self.apply_stroke(stab_x, stab_y) {
                        self.document.dirty_since_last_autosave = true;
                        if self.ui.previous_cursor_position.is_none() {
                            if let UndoRecord::Run {
                                layer_index,
                                color_after,
                                runs,
                                before_pixels,
                                is_alpha_overlay,
                                ..
                            } = stroke
                            {
                                self.undo.init_drag_accumulator(
                                    layer_index,
                                    self.document.canvas.width,
                                    color_after,
                                    is_alpha_overlay,
                                );
                                self.undo.extend_drag_accumulator(runs, before_pixels);
                            }
                        } else if let UndoRecord::Run {
                            runs,
                            before_pixels,
                            ..
                        } = stroke
                        {
                            self.undo.extend_drag_accumulator(runs, before_pixels);
                        }
                    }
                    self.ui.previous_cursor_position = Some((stab_x, stab_y));
                } else {
                    self.ui.previous_cursor_position = Some((raw_x, raw_y));
                }
            }
        } else {
            self.undo.finalize_drag_accumulator();
            self.ui.previous_cursor_position = None;
            self.ui.stabilized_cursor = None;
        }
    }

    /// Apply brush stabilization: compute a virtual cursor position by
    /// lerping toward the real cursor each frame with framerate-independent
    /// exponential ease. On the first drag frame the virtual cursor snaps
    /// to the real position so strokes start immediately.
    ///
    /// # Parameters
    ///
    /// * `raw_x`, `raw_y` — Raw mouse pixel coordinates.
    /// * `dt` — Frame delta time in seconds.
    ///
    /// # Returns
    ///
    /// Stabilized pixel coordinates.
    fn stabilized_pixel(&mut self, raw_x: u32, raw_y: u32, dt: f32) -> (u32, u32) {
        if !self.tool_configuration.stabilization_enabled {
            return (raw_x, raw_y);
        }
        let raw_f = (raw_x as f32, raw_y as f32);
        let st = self.ui.stabilized_cursor.get_or_insert(raw_f);

        if self.ui.previous_cursor_position.is_none() {
            *st = raw_f;
            return (raw_x, raw_y);
        }

        let t = self.tool_configuration.stabilization_smoothing / 100.0;
        let rate = 100.0 * (1.0 - t).max(0.01);
        let lerp_factor = 1.0 - (-rate * dt).exp();
        st.0 += lerp_factor * (raw_f.0 - st.0);
        st.1 += lerp_factor * (raw_f.1 - st.1);

        (st.0.round() as u32, st.1.round() as u32)
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

        let layer = self.document.current_layer;
        let radius = self.tool_configuration.radius;
        let current_tool = self.tool_configuration.current_tool;

        match current_tool {
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
                            BrushStrokeParams::builder(
                                canvas,
                                color,
                                layer,
                                visited,
                                stamp,
                                drag_processed,
                                ds_val,
                            )
                            .start(pixel_x, pixel_y)
                            .end(pixel_x, pixel_y)
                            .alpha_overlay(true)
                            .build(),
                            radius,
                        ))
                    } else {
                        let half_radius = radius;
                        let start_x = pixel_x.saturating_sub(half_radius);
                        let end_x = (pixel_x + half_radius + 1).min(canvas.width);
                        let start_y = pixel_y.saturating_sub(half_radius);
                        let end_y = (pixel_y + half_radius + 1).min(canvas.height);
                        Some(draw_square(
                            start_x, start_y, end_x, end_y, canvas, color, layer, false,
                        ))
                    }
                } else if let Some((previous_x, previous_y)) = previous_position {
                    let stamp = self.undo.next_stamp();
                    let (visited, drag_processed, ds_val) = self.undo.scratch_buffers();
                    Some(draw_square_line(
                        BrushStrokeParams::builder(
                            canvas,
                            color,
                            layer,
                            visited,
                            stamp,
                            drag_processed,
                            ds_val,
                        )
                        .start(previous_x, previous_y)
                        .end(pixel_x, pixel_y)
                        .alpha_overlay(alpha_overlay)
                        .build(),
                        radius,
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
                            BrushStrokeParams::builder(
                                canvas,
                                color,
                                layer,
                                visited,
                                stamp,
                                drag_processed,
                                ds_val,
                            )
                            .start(pixel_x, pixel_y)
                            .end(pixel_x, pixel_y)
                            .alpha_overlay(true)
                            .build(),
                            radius,
                        ))
                    } else {
                        Some(draw_circle(
                            pixel_x, pixel_y, radius, canvas, color, layer, false,
                        ))
                    }
                } else if let Some((previous_x, previous_y)) = previous_position {
                    let stamp = self.undo.next_stamp();
                    let (visited, drag_processed, ds_val) = self.undo.scratch_buffers();
                    Some(draw_circle_line(
                        BrushStrokeParams::builder(
                            canvas,
                            color,
                            layer,
                            visited,
                            stamp,
                            drag_processed,
                            ds_val,
                        )
                        .start(previous_x, previous_y)
                        .end(pixel_x, pixel_y)
                        .alpha_overlay(alpha_overlay)
                        .build(),
                        radius,
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

                self.stamp_library.selected().map(|entry| {
                    let stamp = self.undo.next_stamp();
                    let (visited, drag_processed, ds_val) = self.undo.scratch_buffers();
                    draw_stamp_line(
                        BrushStrokeParams::builder(
                            canvas,
                            color,
                            layer,
                            visited,
                            stamp,
                            drag_processed,
                            ds_val,
                        )
                        .start(start_x, start_y)
                        .end(pixel_x, pixel_y)
                        .alpha_overlay(alpha_overlay)
                        .build(),
                        &entry.pixels,
                        entry.width,
                        entry.height,
                        radius,
                        self.tool_configuration.stamp_tint_mode == StampTintMode::Tinted,
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

                self.brush_library.selected().map(|entry| {
                    let stamp = self.undo.next_stamp();
                    let (visited, drag_processed, ds_val) = self.undo.scratch_buffers();
                    draw_custom_brush_line(
                        BrushStrokeParams::builder(
                            canvas,
                            color,
                            layer,
                            visited,
                            stamp,
                            drag_processed,
                            ds_val,
                        )
                        .start(start_x, start_y)
                        .end(pixel_x, pixel_y)
                        .alpha_overlay(alpha_overlay)
                        .build(),
                        &entry.pixels,
                        entry.width,
                        entry.height,
                        radius,
                        entry.spacing,
                        self.tool_configuration.brush_tint_mode == StampTintMode::Tinted,
                        self.tool_configuration.brush_sampling,
                    )
                })
            }
        }
    }

    /// Draw a persistent status bar below the canvas showing canvas dimensions,
    /// zoom level, and any in-flight file operation (save, export, load, import).
    fn show_canvas_status_line(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.horizontal(|ui| {
            // Left: canvas dimensions.
            ui.label(format!(
                "{}×{}",
                self.document.canvas.width, self.document.canvas.height
            ));
            ui.separator();

            // Center: zoom level.
            ui.label(format!("{:>3.0}%", self.ui.zoom * 100.0));

            // Right: activity status (right-aligned).
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (label, spinner) = match self.ui.progress {
                    ProgressState::Saving => ("Saving…", true),
                    ProgressState::Autosaving => ("Autosaving…", true),
                    ProgressState::Exporting => ("Exporting…", true),
                    ProgressState::Loading => ("Loading…", true),
                    ProgressState::Importing => ("Importing…", true),
                    ProgressState::Idle => {
                        if let Some((msg, expiry)) = &self.ui.last_status_message {
                            if expiry.elapsed() < Duration::from_secs(2) {
                                (*msg, false)
                            } else {
                                self.ui.last_status_message = None;
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                };
                if spinner {
                    ui.add(egui::Spinner::new());
                }
                ui.label(egui::RichText::new(label).color(egui::Color32::from_gray(180)));
            });
        });
    }
}

/// Draw a grid overlay on the canvas, caching shapes across frames.
///
/// Draws vertical and horizontal lines at `grid_size` pixel intervals within the
/// canvas rectangle `canvas_rect`, using screen-space scaling derived from
/// `draw_size`. The shape cache is invalidated when any of `(grid_size, cw, ch)`
/// changes, avoiding per-frame recomputation.
fn draw_grid_overlay(
    ui: &egui::Ui,
    cache: &mut Option<(Vec<egui::Shape>, u32, u32, u32)>,
    cache_key: (u32, u32, u32),
    canvas_rect: Rect,
    draw_size: egui::Vec2,
) {
    let (grid_size, cw, ch) = cache_key;
    let grid_size = grid_size.max(1);

    // Cache hit — reuse previously computed shapes.
    if let Some((shapes, gs, w, h)) = cache {
        if *gs == grid_size && *w == cw && *h == ch {
            ui.painter().extend(shapes.iter().cloned());
            return;
        }
    }

    // Cache miss — (re)build grid line shapes.
    let sx = draw_size.x / cw as f32;
    let sy = draw_size.y / ch as f32;
    let grid_color = egui::Color32::from_gray(128);
    let grid_stroke = egui::Stroke::new(1.0, grid_color);

    let mut shapes = Vec::new();
    let mut x = grid_size as f32;
    while x < cw as f32 {
        let screen_x = canvas_rect.min.x + x * sx;
        shapes.push(egui::Shape::line_segment(
            [
                egui::pos2(screen_x, canvas_rect.top()),
                egui::pos2(screen_x, canvas_rect.bottom()),
            ],
            grid_stroke,
        ));
        x += grid_size as f32;
    }
    let mut y = grid_size as f32;
    while y < ch as f32 {
        let screen_y = canvas_rect.min.y + y * sy;
        shapes.push(egui::Shape::line_segment(
            [
                egui::pos2(canvas_rect.left(), screen_y),
                egui::pos2(canvas_rect.right(), screen_y),
            ],
            grid_stroke,
        ));
        y += grid_size as f32;
    }

    ui.painter().extend(shapes.iter().cloned());
    *cache = Some((shapes, grid_size, cw, ch));
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

#[cfg(test)]
mod tests {
    use eframe::egui;
    use egui_kittest::kittest::Queryable;

    use super::ACTIVE_DURATION_MILLISECONDS;
    use super::CANVAS_BORDER_COLOR;
    use super::CANVAS_BORDER_WIDTH;
    use super::MAX_ZOOM;
    use super::MIN_ZOOM;
    use super::PREVIEW_FILL_ALPHA_FACTOR;
    use super::PREVIEW_STROKE_WIDTH;
    use super::ZOOM_SENSITIVITY;
    use super::compute_canvas_rect;
    use super::draw_circle_preview;
    use super::draw_grid_overlay;
    use super::draw_square_preview;
    use super::stamp_tip_preview;
    use super::uv_to_pixel;
    use super::zoom_around_point;
    use crate::app::ProgressState;

    #[test]
    fn compute_canvas_rect_default_zoom_centers_canvas() {
        let available = egui::vec2(500.0, 500.0);
        let base_size = egui::vec2(200.0, 200.0);
        let (draw_size, rect) = compute_canvas_rect(available, base_size, 1.0, egui::Vec2::ZERO);
        assert_eq!(draw_size, base_size);
        assert_eq!(rect.min, egui::pos2(150.0, 150.0));
        assert_eq!(rect.max, egui::pos2(350.0, 350.0));
    }

    #[test]
    fn compute_canvas_rect_pan_offset_shifts_position() {
        let available = egui::vec2(500.0, 500.0);
        let base_size = egui::vec2(200.0, 200.0);
        let (draw_size, rect) =
            compute_canvas_rect(available, base_size, 1.0, egui::vec2(50.0, -30.0));
        assert_eq!(draw_size, base_size);
        assert_eq!(rect.min, egui::pos2(200.0, 120.0));
    }

    #[test]
    fn compute_canvas_rect_zoom_scales_draw_size() {
        let available = egui::vec2(500.0, 500.0);
        let base_size = egui::vec2(200.0, 200.0);
        let (draw_size, rect) = compute_canvas_rect(available, base_size, 2.0, egui::Vec2::ZERO);
        assert_eq!(draw_size, egui::vec2(400.0, 400.0));
        assert_eq!(rect.min, egui::pos2(50.0, 50.0));
    }

    #[test]
    fn draw_circle_preview_renders_without_panic() {
        egui::__run_test_ui(|ui| {
            let response = ui.allocate_response(egui::vec2(100.0, 100.0), egui::Sense::click());
            draw_circle_preview(
                ui,
                &response,
                5,
                5,
                egui::vec2(100.0, 100.0),
                10,
                egui::Color32::RED,
                100,
                100,
            );
        });
    }

    #[test]
    fn draw_square_preview_renders_without_panic() {
        egui::__run_test_ui(|ui| {
            let response = ui.allocate_response(egui::vec2(100.0, 100.0), egui::Sense::click());
            draw_square_preview(
                ui,
                &response,
                5,
                5,
                egui::vec2(100.0, 100.0),
                10,
                egui::Color32::RED,
                100,
                100,
            );
        });
    }

    #[test]
    fn draw_circle_preview_zero_radius_does_not_panic() {
        egui::__run_test_ui(|ui| {
            let response = ui.allocate_response(egui::vec2(100.0, 100.0), egui::Sense::click());
            draw_circle_preview(
                ui,
                &response,
                50,
                50,
                egui::vec2(100.0, 100.0),
                0,
                egui::Color32::BLUE,
                100,
                100,
            );
        });
    }

    #[test]
    fn draw_square_preview_zero_radius_does_not_panic() {
        egui::__run_test_ui(|ui| {
            let response = ui.allocate_response(egui::vec2(100.0, 100.0), egui::Sense::click());
            draw_square_preview(
                ui,
                &response,
                0,
                0,
                egui::vec2(100.0, 100.0),
                0,
                egui::Color32::GREEN,
                100,
                100,
            );
        });
    }

    #[test]
    fn draw_square_preview_transparent_color_no_panic() {
        egui::__run_test_ui(|ui| {
            let response = ui.allocate_response(egui::vec2(100.0, 100.0), egui::Sense::click());
            draw_square_preview(
                ui,
                &response,
                10,
                10,
                egui::vec2(100.0, 100.0),
                5,
                egui::Color32::TRANSPARENT,
                50,
                50,
            );
        });
    }

    #[test]
    fn preview_fill_alpha_is_between_zero_and_one() {
        assert!(PREVIEW_FILL_ALPHA_FACTOR > 0.0 && PREVIEW_FILL_ALPHA_FACTOR < 1.0);
    }

    #[test]
    fn preview_stroke_width_is_positive() {
        assert!(PREVIEW_STROKE_WIDTH > 0.0);
    }

    #[test]
    fn zoom_sensitivity_is_small_positive() {
        assert!(ZOOM_SENSITIVITY > 0.0 && ZOOM_SENSITIVITY < 1.0);
    }

    #[test]
    fn zoom_bounds_are_valid() {
        assert!(MIN_ZOOM > 0.0);
        assert!(MAX_ZOOM > MIN_ZOOM);
    }

    #[test]
    fn active_duration_is_positive() {
        assert!(ACTIVE_DURATION_MILLISECONDS > 0);
    }

    #[test]
    fn canvas_border_is_valid() {
        assert!(CANVAS_BORDER_WIDTH > 0.0);
        assert_eq!(CANVAS_BORDER_COLOR, egui::Color32::from_rgb(128, 0, 128));
    }

    #[test]
    fn uv_top_left_returns_zero_zero() {
        assert_eq!(uv_to_pixel(egui::Vec2::ZERO, 100, 200), (0, 0));
    }

    #[test]
    fn uv_bottom_right_returns_max_minus_one() {
        assert_eq!(uv_to_pixel(egui::Vec2::new(1.0, 1.0), 100, 200), (99, 199));
    }

    #[test]
    fn uv_center_returns_half_dimensions() {
        assert_eq!(uv_to_pixel(egui::Vec2::new(0.5, 0.5), 100, 200), (50, 100));
    }

    #[test]
    fn uv_quarter_returns_correct_coordinates() {
        assert_eq!(
            uv_to_pixel(egui::Vec2::new(0.25, 0.75), 100, 200),
            (25, 150)
        );
    }

    #[test]
    fn uv_one_by_one_canvas() {
        assert_eq!(uv_to_pixel(egui::Vec2::new(0.0, 0.0), 1, 1), (0, 0));
        assert_eq!(uv_to_pixel(egui::Vec2::new(1.0, 1.0), 1, 1), (0, 0));
    }

    #[test]
    fn uv_clamps_above_one() {
        assert_eq!(uv_to_pixel(egui::Vec2::new(2.0, 3.0), 100, 200), (99, 199));
    }

    #[test]
    fn zoom_unchanged_returns_zero_pan() {
        let result = zoom_around_point(
            1.0,
            1.0,
            egui::pos2(250.0, 250.0),
            egui::pos2(150.0, 150.0),
            egui::vec2(200.0, 200.0),
            egui::vec2(500.0, 500.0),
        );
        assert_eq!(result, egui::Vec2::ZERO);
    }

    #[test]
    fn zoom_in_from_center_returns_zero_pan() {
        let result = zoom_around_point(
            1.0,
            2.0,
            egui::pos2(250.0, 250.0),
            egui::pos2(150.0, 150.0),
            egui::vec2(200.0, 200.0),
            egui::vec2(500.0, 500.0),
        );
        assert_eq!(result, egui::Vec2::ZERO);
    }

    #[test]
    fn zoom_in_from_top_left_adjusts_pan() {
        let result = zoom_around_point(
            1.0,
            2.0,
            egui::pos2(150.0, 150.0),
            egui::pos2(150.0, 150.0),
            egui::vec2(200.0, 200.0),
            egui::vec2(500.0, 500.0),
        );
        assert_eq!(result, egui::vec2(100.0, 100.0));
    }

    #[test]
    fn zoom_out_from_center_returns_zero_pan() {
        let result = zoom_around_point(
            1.0,
            0.5,
            egui::pos2(250.0, 250.0),
            egui::pos2(150.0, 150.0),
            egui::vec2(200.0, 200.0),
            egui::vec2(500.0, 500.0),
        );
        assert_eq!(result, egui::Vec2::ZERO);
    }

    #[test]
    fn zoom_in_from_offset_adjusts_pan_proportionally() {
        let result = zoom_around_point(
            1.0,
            2.0,
            egui::pos2(200.0, 175.0),
            egui::pos2(150.0, 150.0),
            egui::vec2(200.0, 200.0),
            egui::vec2(500.0, 500.0),
        );
        assert_eq!(result, egui::vec2(50.0, 75.0));
    }

    #[test]
    fn stamp_tip_preview_renders_without_panicking() {
        egui::__run_test_ui(|ui| {
            let response = ui.allocate_response(egui::vec2(100.0, 100.0), egui::Sense::click());
            stamp_tip_preview(
                ui,
                &response,
                5,
                5,
                egui::vec2(100.0, 100.0),
                None,
                32,
                32,
                10,
                100,
                100,
                egui::Color32::WHITE,
            );
        });
    }

    #[test]
    fn stamp_tip_preview_with_texture_does_not_panic() {
        egui::__run_test_ui(|ui| {
            let response = ui.allocate_response(egui::vec2(200.0, 200.0), egui::Sense::click());
            stamp_tip_preview(
                ui,
                &response,
                50,
                50,
                egui::vec2(200.0, 200.0),
                None,
                64,
                48,
                20,
                100,
                100,
                egui::Color32::RED,
            );
        });
    }

    #[test]
    fn status_line_shows_default_dimensions_and_zoom() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_canvas_status_line(ui);
        });
        harness.run();

        let _dims = harness.get_by_label("10×10");
        let _zoom = harness.get_by_label("100%");
    }

    #[test]
    fn status_line_shows_saving_progress() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.ui.progress = ProgressState::Saving;

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_canvas_status_line(ui);
        });
        harness.step();

        let _saving = harness.get_by_label("Saving…");
    }

    #[test]
    fn status_line_shows_exporting_progress() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.ui.progress = ProgressState::Exporting;

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_canvas_status_line(ui);
        });
        harness.step();

        let _exporting = harness.get_by_label("Exporting…");
    }

    #[test]
    fn status_line_shows_custom_zoom() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.ui.zoom = 2.5;

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_canvas_status_line(ui);
        });
        harness.run();

        let _zoom = harness.get_by_label("250%");
    }

    #[test]
    fn draw_grid_overlay_disabled_no_panic() {
        egui::__run_test_ui(|ui| {
            let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 100.0));
            let mut cache = None;
            draw_grid_overlay(
                ui,
                &mut cache,
                (10, 100, 100),
                rect,
                egui::vec2(100.0, 100.0),
            );
            // show_grid=false is now handled by the caller — calling with cache means it draws.
            // Test that it doesn't panic.
        });
    }

    #[test]
    fn draw_grid_overlay_enabled_no_panic() {
        egui::__run_test_ui(|ui| {
            let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 100.0));
            let mut cache = None;
            draw_grid_overlay(ui, &mut cache, (10, 50, 50), rect, egui::vec2(100.0, 100.0));
        });
    }

    #[test]
    fn draw_grid_overlay_grid_size_zero_clamped() {
        egui::__run_test_ui(|ui| {
            let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 100.0));
            let mut cache = None;
            draw_grid_overlay(ui, &mut cache, (0, 50, 50), rect, egui::vec2(100.0, 100.0));
        });
    }

    #[test]
    fn stabilized_pixel_disabled_returns_raw() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.tool_configuration.stabilization_enabled = false;

        let (sx, sy) = app.stabilized_pixel(42, 73, 0.016);
        assert_eq!(sx, 42);
        assert_eq!(sy, 73);
    }

    #[test]
    fn stabilized_pixel_first_frame_returns_raw() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.tool_configuration.stabilization_enabled = true;
        // No previous_cursor_position (first frame).

        let (sx, sy) = app.stabilized_pixel(42, 73, 0.016);
        assert_eq!(sx, 42);
        assert_eq!(sy, 73);
    }

    #[test]
    fn stabilized_pixel_lerp_on_subsequent_frame() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.tool_configuration.stabilization_enabled = true;
        app.tool_configuration.stabilization_smoothing = 50.0;
        // Pre-set stabilized cursor to origin, previous cursor to simulate
        // an ongoing drag.
        app.ui.stabilized_cursor = Some((0.0, 0.0));
        app.ui.previous_cursor_position = Some((0, 0));

        let (sx, sy) = app.stabilized_pixel(100, 200, 0.016);
        // Lerp should shift from 0,0 toward 100,200 but not reach it.
        assert!(sx > 0 && sx < 100);
        assert!(sy > 0 && sy < 200);
    }
}
