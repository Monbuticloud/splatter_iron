use crate::brush_params::BrushStrokeParams;
use crate::canvas::Canvas;
use crate::undo::UndoRecord;

pub use crate::tools::circle_brush::draw_circle;
pub use crate::tools::circle_brush::draw_circle_line;

/// Stamp a filled circle on the current layer, capturing before-pixels for undo.
///
/// Delegates to [`circle_brush::draw_circle`] — functionally identical.
/// Pencil is presented as the primary freehand brush distinct from the
/// geometric Circle shape tool.
pub fn draw_pencil(
    center_x: u32,
    center_y: u32,
    radius: u32,
    canvas: &mut Canvas,
    color: eframe::egui::Color32,
    layer: usize,
    alpha_overlay: bool,
) -> UndoRecord {
    draw_circle(center_x, center_y, radius, canvas, color, layer, alpha_overlay)
}

/// Draw a pencil-brush line between two points and return an undo record.
///
/// Delegates to [`circle_brush::draw_circle_line`].
pub fn draw_pencil_line(
    params: BrushStrokeParams<'_>,
    geo_radius: u32,
) -> UndoRecord {
    draw_circle_line(params, geo_radius)
}
