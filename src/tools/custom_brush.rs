//! Custom brush line drawing: [`draw_custom_brush_line`] stamps a brush tip
//! along an interpolated drag path with spacing derived from the brush's
//! native spacing percentage, supporting alpha-overlay blending and the
//! same visited-stamp deduplication as the other brush tools.

use eframe::egui::Color32;

use crate::brush_params::BrushStrokeParams;
use crate::canvas::Canvas;
use crate::tool_configuration::StampSampling;
use crate::tools::stamp_brush::draw_stamp_line;
use crate::undo::UndoRecord;

/// Draw a custom brush tip (or interpolated tip line) onto a canvas layer.
///
/// When `start == end` a single tip is placed.  Otherwise the line is
/// interpolated and the tip is stamped at each step.
///
/// The tip image is scaled so that `radius` controls the output width
/// on the canvas (aspect ratio is preserved).
///
/// The step spacing is computed from the brush's native `spacing_pct`
/// (0–100) and the output tip width: `step = output_w * spacing_pct / 100`.
///
/// # Parameters
///
/// * `params` — Common brush-stroke parameters (coordinates, canvas,
///   colour, layer, visited/drag stamps).
/// * `tip_pixels` — Premultiplied brush-tip pixels (row-major).
/// * `tip_width` — Native width of the tip image.
/// * `tip_height` — Native height of the tip image.
/// * `radius` — Output tip width in canvas pixels.
/// * `spacing_pct` — Spacing percentage (0–100) from the brush metadata.
/// * `tinted` — Multiply tip pixels by `color`.
/// * `sampling` — Pixel-sampling strategy (nearest or bilinear).
///
/// # Panics
///
/// Panics if `params.layer >= params.canvas.pixels.len()`.
pub fn draw_custom_brush_line(
    params: BrushStrokeParams<'_>,
    tip_pixels: &[Color32],
    tip_width: u32,
    tip_height: u32,
    radius: u32,
    spacing_pct: u8,
    tinted: bool,
    sampling: StampSampling,
) -> UndoRecord {
    let BrushStrokeParams {
        start_x,
        start_y,
        end_x,
        end_y,
        canvas,
        color,
        layer,
        visited,
        stamp,
        alpha_overlay,
        drag_processed,
        drag_stamp_value,
    } = params;

    let output_w = radius.max(1);
    let spacing_multiplier = (spacing_pct as f64 / 100.0).max(0.01);
    let step = ((output_w as f64 * spacing_multiplier).round() as u32).max(1);

    let dx = end_x as i64 - start_x as i64;
    let dy = end_y as i64 - start_y as i64;
    let dist_sq = dx * dx + dy * dy;
    let dist = (dist_sq as f64).sqrt();
    let num_steps = if dist_sq == 0 {
        1_usize
    } else {
        ((dist / step as f64).ceil() as usize).max(1)
    };

    let mut all_runs = Vec::new();

    for i in 0..num_steps {
        let t = if num_steps == 1 {
            1.0
        } else {
            (i as f64 + 1.0) / num_steps as f64
        };
        let cx = (start_x as f64 + dx as f64 * t).round() as u32;
        let cy = (start_y as f64 + dy as f64 * t).round() as u32;

        let inner_params = BrushStrokeParams {
            start_x: cx,
            start_y: cy,
            end_x: cx,
            end_y: cy,
            canvas,
            color,
            layer,
            visited,
            stamp,
            alpha_overlay,
            drag_processed,
            drag_stamp_value,
        };
        let record = draw_stamp_line(
            inner_params,
            tip_pixels,
            tip_width,
            tip_height,
            radius,
            tinted,
            sampling,
        );
        let UndoRecord::Run {
            runs: step_runs, ..
        } = record;
        all_runs.extend(step_runs);
    }

    canvas.render_next_frame = true;

    UndoRecord::Run {
        layer_index: layer,
        color_after: color,
        runs: all_runs,
        is_alpha_overlay: alpha_overlay,
    }
}
