//! Stamp brush: [`draw_stamp_line`] for placing an external image onto the
//! canvas at a single point or along an interpolated drag path, with
//! nearest-neighbour or bilinear scaling, colour tinting, and alpha-overlay
//! support.

use eframe::egui::Color32;

use crate::canvas::{ Canvas, DirtyRect };
use crate::stamp_library::StampSampling;
use crate::undo::{ compress_run, RunSegment, UndoRecord };

/// Nearest-neighbour sampling: take the closest source pixel.
fn sample_nearest(
    idx_in_row: usize,
    src_x_map: &[u32],
    src_y: u32,
    stamp_pixels: &[Color32],
    stamp_width: u32,
) -> Color32 {
    let sx = src_x_map[idx_in_row];
    stamp_pixels[(src_y * stamp_width + sx) as usize]
}

/// Bilinear interpolation: sample four surrounding pixels and blend.
fn sample_bilinear(
    _x: u32,
    _y: u32,
    src_x_f: f64,
    src_y_f: f64,
    stamp_pixels: &[Color32],
    stamp_width: u32,
    stamp_height: u32,
) -> Color32 {
    let sx_f = src_x_f;
    let sy_f = src_y_f;
    let x0 = (sx_f.floor() as u32).min(stamp_width - 1);
    let x1 = (x0 + 1).min(stamp_width - 1);
    let y0 = (sy_f.floor() as u32).min(stamp_height - 1);
    let y1 = (y0 + 1).min(stamp_height - 1);
    let fx = sx_f - x0 as f64;
    let fy = sy_f - y0 as f64;

    let top_left = stamp_pixels[(y0 * stamp_width + x0) as usize];
    let top_right = stamp_pixels[(y0 * stamp_width + x1) as usize];
    let bot_left = stamp_pixels[(y1 * stamp_width + x0) as usize];
    let bot_right = stamp_pixels[(y1 * stamp_width + x1) as usize];

    let lerp = |a: u32, b: u32, t: f64| -> u8 { ((a as f64 * (1.0 - t) + b as f64 * t) + 0.5) as u8 };

    // Top row lerp
    let tr = lerp(top_left.r() as u32, top_right.r() as u32, fx);
    let tg = lerp(top_left.g() as u32, top_right.g() as u32, fx);
    let tb = lerp(top_left.b() as u32, top_right.b() as u32, fx);
    let ta = lerp(top_left.a() as u32, top_right.a() as u32, fx);

    // Bottom row lerp
    let br = lerp(bot_left.r() as u32, bot_right.r() as u32, fx);
    let bg = lerp(bot_left.g() as u32, bot_right.g() as u32, fx);
    let bb = lerp(bot_left.b() as u32, bot_right.b() as u32, fx);
    let ba = lerp(bot_left.a() as u32, bot_right.a() as u32, fx);

    // Vertical lerp
    Color32::from_rgba_premultiplied(
        lerp(tr as u32, br as u32, fy),
        lerp(tg as u32, bg as u32, fy),
        lerp(tb as u32, bb as u32, fy),
        lerp(ta as u32, ba as u32, fy),
    )
}

/// Stamp the image once centred at `(center_x, center_y)` and collect runs.
///
/// Computes the output bounding rectangle from `output_w`/`output_h`,
/// clamps to canvas bounds, maps each output pixel back to the source
/// stamp image via nearest-neighbour sampling, applies tint/alpha-overlay,
/// and captures before-pixel data for undo.
#[inline]
#[allow(clippy::too_many_arguments)]
fn stamp_at(
    center_x: u32,
    center_y: u32,
    stamp_pixels: &[Color32],
    stamp_width: u32,
    stamp_height: u32,
    output_w: u32,
    output_h: u32,
    canvas_width: u32,
    canvas_height: u32,
    layer_pixels: &mut [Color32],
    color: Color32,
    alpha_overlay: bool,
    tinted: bool,
    sampling: StampSampling,
    drag_processed: &mut [u32],
    drag_stamp_value: u32,
    runs: &mut Vec<RunSegment>,
    dirty_rect: &mut DirtyRect,
) {
    let half_w = output_w / 2;
    let half_h = output_h / 2;

    // Unclamped output bounds (may be negative)
    let out_left = (center_x as i64) - (half_w as i64);
    let out_top = (center_y as i64) - (half_h as i64);
    let out_right = (center_x as i64) + (half_w as i64);
    let out_bottom = (center_y as i64) + (half_h as i64);

    // Completely off-screen — nothing to do
    if out_right < 0 || out_left >= canvas_width as i64 || out_bottom < 0 || out_top >= canvas_height as i64 {
        return;
    }

    // Clamped visible bounds
    let left = out_left.max(0).min((canvas_width - 1) as i64) as u32;
    let right = out_right.max(0).min((canvas_width - 1) as i64) as u32;
    let top = out_top.max(0).min((canvas_height - 1) as i64) as u32;
    let bottom = out_bottom.max(0).min((canvas_height - 1) as i64) as u32;

    if left > right || top > bottom {
        return;
    }

    let width = canvas_width as usize;
    let color_r = color.r() as u32;
    let color_g = color.g() as u32;
    let color_b = color.b() as u32;
    let color_a = color.a() as u32;

    // Precompute source-x mapping for each output column (O(width) once).
    let src_x_map: Vec<u32> = (left..=right)
        .map(|x| {
            ((((x as i64) - out_left) as f64 * stamp_width as f64 / output_w as f64).round() as u32)
                .min(stamp_width - 1)
        })
        .collect();
    // Also precompute a floating-point version for bilinear.
    let float_scale_x = stamp_width as f64 / output_w as f64;
    let float_scale_y = stamp_height as f64 / output_h as f64;

    for y in top..=bottom {
        let row_start = (y as usize) * width;
        let mut run_start: Option<u32> = None;
        let mut before = Vec::new();

        // Precompute src_y once per row (O(height) vs O(width*height)).
        let src_y = ((((y as i64) - out_top) as f64 * float_scale_y).round() as u32).min(stamp_height - 1);
        let src_y_f = ((y as i64) - out_top) as f64 * float_scale_y;

        for (_x_idx, x) in (left..=right).enumerate() {
            let idx = row_start + x as usize;

            // If already processed in this alpha-overlay drag, close the run
            if alpha_overlay && drag_processed[idx] == drag_stamp_value {
                if let Some(rs) = run_start.take() {
                    let (rle_before, length) = compress_run(std::mem::take(&mut before));
                    runs.push(RunSegment { start: rs, length, before: rle_before });
                }
                continue;
            }

            // Sample from stamp image
            let mut stamp_pixel = match sampling {
                StampSampling::Nearest => {
                    sample_nearest(_x_idx, &src_x_map, src_y, stamp_pixels, stamp_width)
                }
                StampSampling::Bilinear => {
                    let src_x_f = ((x as i64) - out_left) as f64 * float_scale_x;
                    sample_bilinear(
                        _x_idx as u32, 0, src_x_f, src_y_f, stamp_pixels, stamp_width, stamp_height,
                    )
                }
            };

            // Apply tint (component-wise multiply of premultiplied values)
            if tinted {
                stamp_pixel = Color32::from_rgba_premultiplied(
                    ((stamp_pixel.r() as u32 * color_r) / 255) as u8,
                    ((stamp_pixel.g() as u32 * color_g) / 255) as u8,
                    ((stamp_pixel.b() as u32 * color_b) / 255) as u8,
                    ((stamp_pixel.a() as u32 * color_a) / 255) as u8,
                );
            }

            let current = layer_pixels[idx];

            if run_start.is_none() {
                run_start = Some(idx as u32);
            }
            before.push(current);

            layer_pixels[idx] = if alpha_overlay {
                crate::pixel::alpha_blend(current, stamp_pixel)
            } else {
                stamp_pixel
            };

            if alpha_overlay {
                drag_processed[idx] = drag_stamp_value;
            }

            dirty_rect.extend(x, y);
        }

        if let Some(rs) = run_start.take() {
            let (rle_before, length) = compress_run(before);
            runs.push(RunSegment { start: rs, length, before: rle_before });
        }
    }
}

/// Draw a stamp (or interpolated stamp line) onto a canvas layer.
///
/// When `start == end` a single stamp is placed.  Otherwise the line is
/// interpolated and a stamp is placed at each step.
///
/// The stamp image is scaled so that `radius` controls the output width
/// on the canvas (aspect ratio is preserved).
///
/// Sampling mode (nearest-neighbour or bilinear) is controlled by `sampling`.
///
/// # Parameters
///
/// * `start_x` — Column of the line start.
/// * `start_y` — Row of the line start.
/// * `end_x` — Column of the line end.
/// * `end_y` — Row of the line end.
/// * `stamp_pixels` — Premultiplied stamp-image pixels (row-major).
/// * `stamp_width` — Native width of the stamp image.
/// * `stamp_height` — Native height of the stamp image.
/// * `radius` — Output stamp width in canvas pixels.
/// * `canvas` — The canvas to draw on.
/// * `color` — Tool colour (premultiplied); used for tinting.
/// * `layer` — Index of the target layer.
/// * `alpha_overlay` — Alpha-blend instead of overwriting.
/// * `tinted` — Multiply stamp pixels by `color`.
/// * `sampling` — Pixel-sampling strategy (nearest or bilinear).
/// * `drag_processed` — Per-pixel drag-scoped dedup buffer.
/// * `drag_stamp_value` — Current drag-scoped stamp value.
///
/// # Panics
///
/// Panics if `layer >= canvas.pixels.len()`.
#[inline]
pub fn draw_stamp_line(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    stamp_pixels: &[Color32],
    stamp_width: u32,
    stamp_height: u32,
    radius: u32,
    canvas: &mut Canvas,
    color: Color32,
    layer: usize,
    alpha_overlay: bool,
    tinted: bool,
    sampling: StampSampling,
    drag_processed: &mut [u32],
    drag_stamp_value: u32,
) -> UndoRecord {
    let canvas_width = canvas.width;
    let canvas_height = canvas.height;
    let layer_pixels = &mut canvas.pixels[layer].pixels;

    let output_w = radius.max(1);
    let output_h = ((stamp_height as f64 * output_w as f64 / stamp_width as f64).round() as u32).max(1);

    // Dynamic step spacing: opaque mode stamps edge-to-edge; alpha overlay
    // stamps half-overlapping for smoother blends.
    let step = if alpha_overlay { (output_w / 2).max(1) } else { output_w.max(1) };

    let mut runs: Vec<RunSegment> = Vec::new();
    let mut dirty_rect = DirtyRect::empty();

    let dx = end_x as i64 - start_x as i64;
    let dy = end_y as i64 - start_y as i64;
    let dist_sq = dx * dx + dy * dy;
    let dist = (dist_sq as f64).sqrt();
    let num_steps = if dist_sq == 0 {
        1_usize
    } else {
        ((dist / step as f64).ceil() as usize).max(1)
    };

    for i in 0..num_steps {
        let t = if num_steps == 1 {
            1.0
        } else {
            (i as f64 + 1.0) / num_steps as f64
        };
        let cx = (start_x as f64 + dx as f64 * t).round() as u32;
        let cy = (start_y as f64 + dy as f64 * t).round() as u32;

        stamp_at(
            cx,
            cy,
            stamp_pixels,
            stamp_width,
            stamp_height,
            output_w,
            output_h,
            canvas_width,
            canvas_height,
            layer_pixels,
            color,
            alpha_overlay,
            tinted,
            sampling,
            drag_processed,
            drag_stamp_value,
            &mut runs,
            &mut dirty_rect,
        );
    }

    canvas.dirty_rect = match canvas.dirty_rect {
        Some(rect) => Some(rect.union(&dirty_rect)),
        None => Some(dirty_rect),
    };

    UndoRecord::Run {
        layer_index: layer,
        color_after: color,
        runs,
        is_alpha_overlay: alpha_overlay,
    }
}
