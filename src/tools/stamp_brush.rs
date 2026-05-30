//! Stamp brush: \[`draw_stamp_line`\] for placing an external image onto the
//! canvas at a single point or along an interpolated drag path, with
//! nearest-neighbour or bilinear scaling, colour tinting, and alpha-overlay
//! support.

use eframe::egui::Color32;

use crate::brush_params::BrushStrokeParams;
use crate::canvas::DirtyRect;
use crate::tool_configuration::StampSampling;
use crate::undo::RunSegment;
use crate::undo::UndoRecord;
use crate::undo::compress_run;

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

    let lerp =
        |a: u32, b: u32, t: f64| -> u8 { ((a as f64 * (1.0 - t) + b as f64 * t) + 0.5) as u8 };

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
///
/// Uses a `visited` buffer per-stroke (combined with `stamp`) to avoid
/// re-processing pixels already covered by an earlier stamp position in
/// the same stroke, and a `drag_processed` buffer per-drag-gesture to
/// avoid re-painting alpha-overlay pixels across frames.
// SAFETY: 18 parameters are intentional — `stamp_at` is the hottest inner
// loop for stamp rendering; collecting into a struct would add allocation
// overhead on every pixel. This is one of two codebase-wide exceptions
// documented in AGENTS.md.
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
    visited: &mut [u32],
    stamp: u32,
    drag_processed: &mut [u32],
    drag_stamp_value: u32,
    runs: &mut Vec<RunSegment>,
    scratch_src_x: &mut Vec<u32>,
    dirty_rect: &mut DirtyRect,
) {
    let half_w = output_w / 2;
    let half_h = output_h / 2;

    // Unclamped output bounds (may be negative).
    // Compute so the inclusive range spans exactly output_w × output_h.
    let out_left = (center_x as i64) - (half_w as i64);
    let out_top = (center_y as i64) - (half_h as i64);
    let out_right = out_left + output_w as i64 - 1;
    let out_bottom = out_top + output_h as i64 - 1;

    // Completely off-screen — nothing to do
    if out_right < 0
        || out_left >= canvas_width as i64
        || out_bottom < 0
        || out_top >= canvas_height as i64
    {
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
    scratch_src_x.clear();
    scratch_src_x.reserve((right - left + 1) as usize);
    scratch_src_x.extend((left..=right).map(|x| {
        ((((x as i64) - out_left) as f64 * stamp_width as f64 / output_w as f64).round() as u32)
            .min(stamp_width - 1)
    }));
    // Also precompute a floating-point version for bilinear.
    let float_scale_x = stamp_width as f64 / output_w as f64;
    let float_scale_y = stamp_height as f64 / output_h as f64;

    for y in top..=bottom {
        let row_start = (y as usize) * width;
        let mut run_start: Option<u32> = None;
        let mut before = Vec::new();

        // Precompute src_y once per row (O(height) vs O(width*height)).
        let src_y =
            ((((y as i64) - out_top) as f64 * float_scale_y).round() as u32).min(stamp_height - 1);
        let src_y_f = ((y as i64) - out_top) as f64 * float_scale_y;

        for (_x_idx, x) in (left..=right).enumerate() {
            let idx = row_start + x as usize;

            // Skip pixels already painted by an overlapping stamp in this stroke
            if visited[idx] == stamp {
                if let Some(rs) = run_start.take() {
                    let (rle_before, length) = compress_run(std::mem::take(&mut before));
                    runs.push(RunSegment {
                        start: rs,
                        length,
                        before: rle_before,
                    });
                }
                continue;
            }

            // If already processed in this alpha-overlay drag, close the run
            if alpha_overlay && drag_processed[idx] == drag_stamp_value {
                if let Some(rs) = run_start.take() {
                    let (rle_before, length) = compress_run(std::mem::take(&mut before));
                    runs.push(RunSegment {
                        start: rs,
                        length,
                        before: rle_before,
                    });
                }
                continue;
            }

            // Sample from stamp image
            let mut stamp_pixel = match sampling {
                StampSampling::Nearest => sample_nearest(
                    _x_idx,
                    scratch_src_x.as_slice(),
                    src_y,
                    stamp_pixels,
                    stamp_width,
                ),
                StampSampling::Bilinear => {
                    let src_x_f = ((x as i64) - out_left) as f64 * float_scale_x;
                    sample_bilinear(src_x_f, src_y_f, stamp_pixels, stamp_width, stamp_height)
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

            visited[idx] = stamp;

            if alpha_overlay {
                drag_processed[idx] = drag_stamp_value;
            }

            dirty_rect.extend(x, y);
        }

        if let Some(rs) = run_start.take() {
            let (rle_before, length) = compress_run(before);
            runs.push(RunSegment {
                start: rs,
                length,
                before: rle_before,
            });
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
/// * `params` — Common brush-stroke parameters (coordinates, canvas,
///   colour, layer, visited/drag stamps).
/// * `stamp_pixels` — Premultiplied stamp-image pixels (row-major).
/// * `stamp_width` — Native width of the stamp image.
/// * `stamp_height` — Native height of the stamp image.
/// * `radius` — Output stamp width in canvas pixels.
/// * `tinted` — Multiply stamp pixels by `color`.
/// * `sampling` — Pixel-sampling strategy (nearest or bilinear).
///
/// # Returns
///
/// An empty `UndoRecord` (no-op) when `stamp_width` or `stamp_height` is zero.
///
/// # Panics
///
/// Panics if `params.layer >= params.canvas.pixels.len()`.
#[inline]
pub fn draw_stamp_line(
    params: BrushStrokeParams<'_>,
    stamp_pixels: &[Color32],
    stamp_width: u32,
    stamp_height: u32,
    radius: u32,
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

    let canvas_width = canvas.width;
    let canvas_height = canvas.height;
    let layer_pixels = &mut canvas.pixels[layer].pixels;

    if stamp_width == 0 || stamp_height == 0 {
        return UndoRecord::Run {
            layer_index: layer,
            color_after: color,
            runs: Vec::new(),
            is_alpha_overlay: alpha_overlay,
        };
    }

    let output_w = radius.max(1);
    let output_h =
        ((stamp_height as f64 * output_w as f64 / stamp_width as f64).round() as u32).max(1);

    // Dynamic step spacing: opaque mode stamps edge-to-edge; alpha overlay
    // stamps half-overlapping for smoother blends.
    let step = if alpha_overlay {
        (output_w / 2).max(1)
    } else {
        output_w.max(1)
    };

    let mut runs: Vec<RunSegment> = Vec::new();
    let mut scratch_src_x: Vec<u32> = Vec::new();
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
            visited,
            stamp,
            drag_processed,
            drag_stamp_value,
            &mut runs,
            &mut scratch_src_x,
            &mut dirty_rect,
        );
    }

    canvas.dirty_rect.add(dirty_rect);

    UndoRecord::Run {
        layer_index: layer,
        color_after: color,
        runs,
        is_alpha_overlay: alpha_overlay,
    }
}
