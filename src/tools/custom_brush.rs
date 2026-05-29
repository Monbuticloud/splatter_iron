//! Custom brush line drawing: [`draw_custom_brush_line`] stamps a brush tip
//! along an interpolated drag path with spacing derived from the brush's
//! native spacing percentage, supporting alpha-overlay blending and the
//! same visited-stamp deduplication as the other brush tools.

use eframe::egui::Color32;

use crate::canvas::Canvas;
use crate::stamp_library::StampSampling;
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
/// * `start_x` — Column of the line start.
/// * `start_y` — Row of the line start.
/// * `end_x` — Column of the line end.
/// * `end_y` — Row of the line end.
/// * `tip_pixels` — Premultiplied brush-tip pixels (row-major).
/// * `tip_width` — Native width of the tip image.
/// * `tip_height` — Native height of the tip image.
/// * `radius` — Output tip width in canvas pixels.
/// * `spacing_pct` — Spacing percentage (0–100) from the brush metadata.
/// * `canvas` — The canvas to draw on.
/// * `color` — Tool colour (premultiplied); used for tinting.
/// * `layer` — Index of the target layer.
/// * `visited` — Per-stroke pixel dedup buffer.
/// * `stamp` — Stroke-scoped stamp value for the visited buffer.
/// * `alpha_overlay` — Alpha-blend instead of overwriting.
/// * `tinted` — Multiply tip pixels by `color`.
/// * `sampling` — Pixel-sampling strategy (nearest or bilinear).
/// * `drag_processed` — Per-pixel drag-scoped dedup buffer.
/// * `drag_stamp_value` — Current drag-scoped stamp value.
///
/// # Panics
///
/// Panics if `layer >= canvas.pixels.len()`.
pub fn draw_custom_brush_line(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    tip_pixels: &[Color32],
    tip_width: u32,
    tip_height: u32,
    radius: u32,
    spacing_pct: u8,
    canvas: &mut Canvas,
    color: Color32,
    layer: usize,
    visited: &mut [u32],
    stamp: u32,
    alpha_overlay: bool,
    tinted: bool,
    sampling: StampSampling,
    drag_processed: &mut [u32],
    drag_stamp_value: u32,
) -> UndoRecord {
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

        let record = draw_stamp_line(
            cx,
            cy,
            cx,
            cy,
            tip_pixels,
            tip_width,
            tip_height,
            radius,
            canvas,
            color,
            layer,
            visited,
            stamp,
            alpha_overlay,
            tinted,
            sampling,
            drag_processed,
            drag_stamp_value,
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
