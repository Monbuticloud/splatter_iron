//! Rectangular fill brush: \[`draw_square`\] for single stamps and
//! \[`draw_square_line`\] for Bresenham-interpolated strokes with
//! visited-stamp deduplication.

use eframe::egui::Color32;
use eframe::egui::{self};

use crate::brush_params::BrushStrokeParams;
use crate::canvas::Canvas;
use crate::canvas::DirtyRect;
use crate::undo::RunSegment;
use crate::undo::UndoRecord;
use crate::undo::compress_and_store;

/// Fill a rectangular region of a pixel slice without capturing undo data.
///
/// # Panics
///
/// Panics if `pixels` is not large enough to cover the rectangle at the given width.
#[inline]
fn fill_square_impl(
    pixels: &mut [Color32],
    width: usize,
    start_x: u32,
    end_x: u32,
    start_y: u32,
    end_y: u32,
    color: Color32,
    alpha_overlay: bool,
) {
    if alpha_overlay {
        for y in start_y..end_y {
            let row_start = (y as usize) * width;
            let start = row_start + (start_x as usize);
            let end = row_start + (end_x as usize);
            crate::pixel::alpha_blend_span(&mut pixels[start..end], color);
        }
    } else {
        for y in start_y..end_y {
            let row_start = (y as usize) * width;
            let start = row_start + (start_x as usize);
            let end = row_start + (end_x as usize);
            pixels[start..end].fill(color);
        }
    }
}

/// Mark all pixel indices covered by a brush stroke line in the `visited` buffer.
///
/// Uses the Bresenham line algorithm to step along the line. At each step the
/// square brush footprint is stamped, but per-row span tracking
/// (`row_min_x`/`row_max_x`) ensures each pixel is written only once even when
/// consecutive brush positions heavily overlap. The caller can later scan
/// `visited` for values matching `stamp` to get deduplicated, sorted positions.
///
/// Clamps brush bounds to canvas dimensions. Tracks the bounding box of
/// stamped pixels via `dirty_rect`.
///
/// # Panics
///
/// Panics if `visited` is shorter than `width * height`, since every
/// pixel index along the stamped line is written directly into the
/// visited buffer without a bounds check.
#[inline]
fn stamp_line_positions(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    brush_radius: u32,
    width: usize,
    height: u32,
    visited: &mut [u32],
    stamp: u32,
    dirty_rect: &mut DirtyRect,
) {
    let half_radius = brush_radius;

    // Per-row span tracking: each row's min_x/max_x of already-stamped columns.
    // u32::MAX / 0 = unstamped. This avoids re-stamping pixels already covered
    // by a previous Bresenham step (common case for overlapping brush footprints).
    let mut row_min_x = vec![u32::MAX; height as usize];
    let mut row_max_x = vec![0u32; height as usize];

    let mut current_x = start_x as i32;
    let mut current_y = start_y as i32;
    let target_x = end_x as i32;
    let target_y = end_y as i32;

    let delta_x = target_x.abs_diff(current_x) as i32;
    let step_x = if current_x < target_x { 1 } else { -1 };
    let delta_y = -(target_y.abs_diff(current_y) as i32);
    let step_y = if current_y < target_y { 1 } else { -1 };
    let mut error = delta_x + delta_y;

    loop {
        let brush_start_x = current_x
            .saturating_sub(half_radius as i32)
            .max(0)
            .min((width as i32) - 1) as u32;
        let brush_end_x = current_x
            .saturating_add(half_radius as i32)
            .saturating_add(1)
            .max(0)
            .min(width as i32) as u32;
        let brush_start_y = current_y
            .saturating_sub(half_radius as i32)
            .max(0)
            .min((height as i32) - 1) as u32;
        let brush_end_y = current_y
            .saturating_add(half_radius as i32)
            .saturating_add(1)
            .max(0)
            .min(height as i32) as u32;

        dirty_rect.extend(brush_start_x, brush_start_y);
        dirty_rect.extend(brush_end_x - 1, brush_end_y - 1);

        for y in brush_start_y..brush_end_y {
            let row = y as usize;
            let row_start = row * width;
            let cur_min = row_min_x[row];
            let cur_max = row_max_x[row];

            if cur_min == u32::MAX {
                // First time this row is hit — stamp the full span.
                for x in brush_start_x..brush_end_x {
                    visited[row_start + x as usize] = stamp;
                }
                row_min_x[row] = brush_start_x;
                row_max_x[row] = brush_end_x;
            } else {
                // Stamp newly-covered left extension.
                if brush_start_x < cur_min {
                    for x in brush_start_x..cur_min {
                        visited[row_start + x as usize] = stamp;
                    }
                    row_min_x[row] = brush_start_x;
                }
                // Stamp newly-covered right extension.
                if brush_end_x > cur_max {
                    for x in cur_max..brush_end_x {
                        visited[row_start + x as usize] = stamp;
                    }
                    row_max_x[row] = brush_end_x;
                }
            }
        }

        if current_x == target_x && current_y == target_y {
            break;
        }
        let error_times_two = error.saturating_mul(2);
        if error_times_two >= delta_y {
            error += delta_y;
            current_x += step_x;
        }
        if error_times_two <= delta_x {
            error += delta_x;
            current_y += step_y;
        }
    }
}

/// Draw a filled rectangle on a canvas layer and return an undo record.
///
/// Coordinates are clamped to canvas bounds. If `start_x >= end_x` or
/// `start_y >= end_y` the call is a no-op and returns an empty undo record.
/// Captures before-pixel data for every touched position to support undo.
///
/// # Parameters
///
/// * `start_x` — Left column of the rectangle (inclusive).
/// * `start_y` — Top row of the rectangle (inclusive).
/// * `end_x` — Right column of the rectangle (exclusive).
/// * `end_y` — Bottom row of the rectangle (exclusive).
/// * `canvas` — The canvas whose pixels will be modified.
/// * `color` — Fill colour (premultiplied-alpha).
/// * `layer` — Index of the target layer.
/// * `alpha_overlay` — Whether to alpha-blend instead of overwriting.
///
/// # Panics
///
/// Panics if `layer >= canvas.pixels.len()`.
#[inline]
pub fn draw_square(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    canvas: &mut Canvas,
    color: egui::Color32,
    layer: usize,
    alpha_overlay: bool,
) -> UndoRecord {
    let width = canvas.width as usize;
    let height = canvas.height;

    // Clamp once outside the hot loop
    let start_x = start_x.min(canvas.width);
    let end_x = end_x.min(canvas.width);
    let start_y = start_y.min(height);
    let end_y = end_y.min(height);

    // Early out
    if start_x >= end_x || start_y >= end_y {
        return UndoRecord::Run {
            layer_index: layer,
            color_after: color,
            runs: Vec::new(),
            before_pixels: Vec::new(),
            compressed_before_pixels: None,
            is_alpha_overlay: alpha_overlay,
            full_layer_before: None,
        };
    }

    let pixels = &mut canvas.pixels[layer].pixels;

    // Capture runs (one per row) and fill in one pass
    let mut runs = Vec::with_capacity((end_y - start_y) as usize);
    let mut before_pixels = Vec::new();

    for y in start_y..end_y {
        let row_start = (y as usize) * width;
        let start = row_start + (start_x as usize);
        let end = row_start + (end_x as usize);

        let (before, length) = compress_and_store(&pixels[start..end], &mut before_pixels);
        runs.push(RunSegment {
            start: start as u32,
            length,
            before,
        });
    }

    // Fill the rectangle (efficient contiguous write)
    fill_square_impl(
        pixels,
        width,
        start_x,
        end_x,
        start_y,
        end_y,
        color,
        alpha_overlay,
    );

    let rect = DirtyRect::new(start_x, start_y, end_x - 1, end_y - 1);
    canvas.dirty_rect.add(rect);

    UndoRecord::Run {
        layer_index: layer,
        color_after: color,
        runs,
        before_pixels,
        compressed_before_pixels: None,
        is_alpha_overlay: alpha_overlay,
        full_layer_before: None,
    }
}

/// Draw a brush line between two points and return an undo record.
///
/// Uses `stamp_line_positions` to find all touched pixels, then applies the
/// color and captures before-data for undo.  The `visited` buffer and `stamp`
/// value must be managed by the caller to avoid re-processing old stamps.
///
/// # Parameters
///
/// * `params` — Common brush-stroke parameters (coordinates, canvas,
///   colour, layer, visited/drag stamps).
/// * `brush_radius` — Brush radius in pixels.
///
/// # Panics
///
/// Panics if `params.layer >= params.canvas.pixels.len()`.
#[inline]
pub fn draw_square_line(params: BrushStrokeParams<'_>, brush_radius: u32) -> UndoRecord {
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

    let width = canvas.width as usize;
    let height = canvas.height;

    let mut dirty_rect = DirtyRect::empty();
    stamp_line_positions(
        start_x,
        start_y,
        end_x,
        end_y,
        brush_radius,
        width,
        height,
        visited,
        stamp,
        &mut dirty_rect,
    );

    let pixels = &mut canvas.pixels[layer].pixels;
    let mut before_pixels = Vec::new();

    let runs = crate::tools::brush_common::apply_visited_runs(
        pixels,
        &dirty_rect,
        width,
        visited,
        stamp,
        color,
        alpha_overlay,
        drag_processed,
        drag_stamp_value,
        &mut before_pixels,
    );

    canvas.dirty_rect.add(dirty_rect);

    UndoRecord::Run {
        layer_index: layer,
        color_after: color,
        runs,
        before_pixels,
        compressed_before_pixels: None,
        is_alpha_overlay: alpha_overlay,
        full_layer_before: None,
    }
}
