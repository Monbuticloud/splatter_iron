//! Midpoint-circle span-fill brush: [`draw_circle`] for single stamps and
//! [`draw_circle_line`] for Bresenham-interpolated strokes with
//! visited-stamp deduplication.

use eframe::egui::{self, Color32};

use crate::canvas::{ Canvas, DirtyRect };
use crate::undo::{ compress_run, RunSegment, UndoRecord };

/// Fill a circular region without capturing undo data.
///
/// Uses midpoint-circle span filling: for each dy in `0..=radius`, computes
/// `dx = sqrt(r² − dy²)` and fills rows `center_y ± dy` from `center_x - dx`
/// to `center_x + dx` as contiguous slices.  Rows that fall outside the canvas
/// (due to the circle being near an edge) are silently skipped.
///
/// The caller is responsible for clamping `(center_x, center_y)` to canvas
/// dimensions; out-of-bounds row/column indices are handled gracefully.
///
/// # Panics
///
/// Panics if `pixels.len() < canvas_width * canvas_height`.
#[inline]
fn fill_circle_impl(
    pixels: &mut [Color32],
    width: usize,
    center_x: u32,
    center_y: u32,
    radius: u32,
    color: Color32,
    canvas_width: u32,
    canvas_height: u32,
    alpha_overlay: bool,
) {
    if radius == 0 {
        let pixel_index = (center_y as usize) * width + center_x as usize;
        if alpha_overlay {
            pixels[pixel_index] = crate::pixel::alpha_blend(pixels[pixel_index], color);
        } else {
            pixels[pixel_index] = color;
        }
        return;
    }

    let radius_squared = (radius as u64) * (radius as u64);

    for delta_y in 0..=radius {
        let delta_y_squared = (delta_y as u64) * (delta_y as u64);
        let delta_x = ((radius_squared - delta_y_squared) as f64).sqrt() as u32;
        let span_start = center_x.saturating_sub(delta_x).min(canvas_width - 1);
        let span_end = (center_x + delta_x).min(canvas_width - 1);
        if span_start > span_end {
            continue;
        }

        let apply = |span: &mut [Color32]| {
            if alpha_overlay {
                for pixel in span.iter_mut() {
                    *pixel = crate::pixel::alpha_blend(*pixel, color);
                }
            } else {
                span.fill(color);
            }
        };

        // Top half row
        if let Some(y) = center_y.checked_sub(delta_y) {
            let row_start = (y as usize) * width;
            apply(&mut pixels[row_start + span_start as usize..=row_start + span_end as usize]);
        }

        // Bottom half (skip centre-row duplicate)
        if delta_y != 0 {
            let y = center_y + delta_y;
            if y < canvas_height {
                let row_start = (y as usize) * width;
                apply(&mut pixels[row_start + span_start as usize..=row_start + span_end as usize]);
            }
        }
    }
}

/// Draw a filled circle on a canvas layer and return an undo record.
///
/// Clamps the brush footprint to canvas bounds.  Captures before-pixel data
/// for every touched position to support undo.  If the circle has no visible
/// pixels after clamping, returns an empty undo record.
///
/// # Parameters
///
/// * `center_x` — Column of the circle centre.
/// * `center_y` — Row of the circle centre.
/// * `radius` — Circle radius in pixels.
/// * `canvas` — The canvas whose pixels will be modified.
/// * `color` — Fill colour (premultiplied-alpha).
/// * `layer` — Index of the target layer.
/// * `alpha_overlay` — Whether to alpha-blend instead of overwriting.
///
/// # Panics
///
/// Panics if `layer >= canvas.pixels.len()`.
#[inline]
pub fn draw_circle(
    center_x: u32,
    center_y: u32,
    radius: u32,
    canvas: &mut Canvas,
    color: egui::Color32,
    layer: usize,
    alpha_overlay: bool,
) -> UndoRecord {
    let width = canvas.width as usize;
    let height = canvas.height;

    // Clamp center to canvas
    let center_x = center_x.min(canvas.width);
    let center_y = center_y.min(height);

    if radius == 0 || center_x >= canvas.width || center_y >= height {
        return UndoRecord::Run {
            layer_index: layer,
            color_after: color,
            runs: Vec::new(),
            is_alpha_overlay: alpha_overlay,
        };
    }

    let pixels = &mut canvas.pixels[layer].pixels;
    let mut runs: Vec<RunSegment> = Vec::new();

    let radius_squared = (radius as u64) * (radius as u64);

    for delta_y in 0..=radius {
        let delta_y_squared = (delta_y as u64) * (delta_y as u64);
        let delta_x = ((radius_squared - delta_y_squared) as f64).sqrt() as u32;
        let span_start = center_x.saturating_sub(delta_x).min(canvas.width - 1);
        let span_end = (center_x + delta_x).min(canvas.width - 1);

        if span_start > span_end {
            continue;
        }

        // Top half row
        if let Some(y) = center_y.checked_sub(delta_y) {
            let row_start = (y as usize) * width;
            let start = row_start + span_start as usize;
            let end = row_start + span_end as usize + 1;
            let mut before = Vec::with_capacity(end - start);
            before.extend_from_slice(&pixels[start..end]);
            let (before, length) = compress_run(before);
            runs.push(RunSegment {
                start: start as u32,
                length,
                before,
            });
        }

        // Bottom half (skip centre-row duplicate)
        if delta_y != 0 {
            let y = center_y + delta_y;
            if y < height {
                let row_start = (y as usize) * width;
                let start = row_start + span_start as usize;
                let end = row_start + span_end as usize + 1;
                let mut before = Vec::with_capacity(end - start);
                before.extend_from_slice(&pixels[start..end]);
                let (before, length) = compress_run(before);
                runs.push(RunSegment {
                    start: start as u32,
                    length,
                    before,
                });
            }
        }
    }

    // Fill the circle
    fill_circle_impl(pixels, width, center_x, center_y, radius, color, canvas.width, height, alpha_overlay);

    let circle_min_x = center_x.saturating_sub(radius);
    let circle_min_y = center_y.saturating_sub(radius);
    let circle_max_x = (center_x + radius).min(canvas.width - 1);
    let circle_max_y = (center_y + radius).min(canvas.height - 1);
    let rect = DirtyRect::new(circle_min_x, circle_min_y, circle_max_x, circle_max_y);
    canvas.dirty_rect = match canvas.dirty_rect {
        Some(rectangle) => Some(rectangle.union(&rect)),
        None => Some(rect),
    };

    UndoRecord::Run {
        layer_index: layer,
        color_after: color,
        runs,
        is_alpha_overlay: alpha_overlay,
    }
}

/// Mark all pixel indices covered by a circle-brush stroke line in the `visited` buffer.
///
/// Uses the Bresenham line algorithm to step along the line and stamps every
/// pixel within the circle radius (geometric radius) at each step.
/// The caller can later scan `visited` for values matching `stamp` to get
/// deduplicated, sorted positions.
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
fn stamp_circle_positions(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    geo_radius: u32,
    width: usize,
    height: u32,
    visited: &mut [u32],
    stamp: u32,
    dirty_rect: &mut DirtyRect,
) {
    if geo_radius == 0 {
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
            if current_x >= 0 && (current_x as u32) < width as u32 && current_y >= 0 && (current_y as u32) < height {
                let x = current_x as u32;
                let y = current_y as u32;
                visited[(y as usize) * width + x as usize] = stamp;
                dirty_rect.extend(x, y);
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
        return;
    }

    let mut current_x = start_x as i32;
    let mut current_y = start_y as i32;
    let target_x = end_x as i32;
    let target_y = end_y as i32;

    let delta_x = target_x.abs_diff(current_x) as i32;
    let step_x = if current_x < target_x { 1 } else { -1 };
    let delta_y = -(target_y.abs_diff(current_y) as i32);
    let step_y = if current_y < target_y { 1 } else { -1 };
    let mut error = delta_x + delta_y;

    let radius_squared = (geo_radius as u64) * (geo_radius as u64);

    loop {
        let geo_radius_i32 = geo_radius as i32;
        let circle_min_y = (current_y - geo_radius_i32).max(0) as u32;
        let circle_max_y = (current_y + geo_radius_i32).min(height as i32 - 1).max(0) as u32;
        let circle_min_x = (current_x - geo_radius_i32).max(0) as u32;
        let circle_max_x = (current_x + geo_radius_i32).min(width as i32 - 1).max(0) as u32;
        dirty_rect.extend(circle_min_x, circle_min_y);
        dirty_rect.extend(circle_max_x, circle_max_y);

        for delta_y in -geo_radius_i32..=geo_radius_i32 {
            let y = current_y + delta_y;
            if y < 0 || y >= height as i32 {
                continue;
            }
            let delta_y_squared = (delta_y.abs() as u64) * (delta_y.abs() as u64);
            let delta_x = ((radius_squared - delta_y_squared) as f64).sqrt() as i32;
            let row_start = (y as usize) * width;
            let start_x_local = (current_x - delta_x).max(0).min(width as i32 - 1) as usize;
            let end_x_local = (current_x + delta_x).max(0).min(width as i32 - 1) as usize;
            for pixel_index in start_x_local..=end_x_local {
                visited[row_start + pixel_index] = stamp;
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

/// Draw a circle-brush line between two points and return an undo record.
///
/// Uses `stamp_circle_positions` to find all touched pixels, then applies the
/// color and captures before-data for undo.  The `visited` buffer and `stamp`
/// value must be managed by the caller to avoid re-processing old stamps.
///
/// # Parameters
///
/// * `start_x` — Column of the line start point.
/// * `start_y` — Row of the line start point.
/// * `end_x` — Column of the line end point.
/// * `end_y` — Row of the line end point.
/// * `geo_radius` — Brush radius in pixels.
/// * `canvas` — The canvas whose pixels will be modified.
/// * `color` — Stroke colour (premultiplied-alpha).
/// * `layer` — Index of the target layer.
/// * `visited` — Stamp buffer for pixel deduplication.
/// * `stamp` — Current stamp value for this stroke.
/// * `alpha_overlay` — Whether to alpha-blend instead of overwriting.
/// * `drag_processed` — Drag-scoped deduplication buffer.
/// * `drag_stamp_value` — Current drag stamp value.
///
/// # Panics
///
/// Panics if `layer >= canvas.pixels.len()`.
#[inline]
pub fn draw_circle_line(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    geo_radius: u32,
    canvas: &mut Canvas,
    color: egui::Color32,
    layer: usize,
    visited: &mut [u32],
    stamp: u32,
    alpha_overlay: bool,
    drag_processed: &mut [u32],
    drag_stamp_value: u32,
) -> UndoRecord {
    let width = canvas.width as usize;
    let height = canvas.height;

    let mut dirty_rect = DirtyRect::empty();
    stamp_circle_positions(
        start_x,
        start_y,
        end_x,
        end_y,
        geo_radius,
        width,
        height,
        visited,
        stamp,
        &mut dirty_rect,
    );

    let pixels = &mut canvas.pixels[layer].pixels;

    let mut runs: Vec<RunSegment> = Vec::new();
    for y in dirty_rect.min_y..=dirty_rect.max_y {
        let row_start = (y as usize) * width;
        let mut x = dirty_rect.min_x;
        while x <= dirty_rect.max_x {
            let pixel_index = row_start + x as usize;
            if visited[pixel_index] != stamp {
                x += 1;
                continue;
            }
            if alpha_overlay && drag_processed[pixel_index] == drag_stamp_value {
                x += 1;
                continue;
            }
            let run_start = pixel_index as u32;
            let mut before = Vec::new();
            while x <= dirty_rect.max_x {
                let next_pixel_index = row_start + x as usize;
                if visited[next_pixel_index] != stamp {
                    break;
                }
                if alpha_overlay && drag_processed[next_pixel_index] == drag_stamp_value {
                    break;
                }
                before.push(pixels[next_pixel_index]);
                pixels[next_pixel_index] = if alpha_overlay {
                    crate::pixel::alpha_blend(pixels[next_pixel_index], color)
                } else {
                    color
                };
                if alpha_overlay {
                    drag_processed[next_pixel_index] = drag_stamp_value;
                }
                x += 1;
            }
            let (rle_before, length) = compress_run(before);
            runs.push(RunSegment { start: run_start, length, before: rle_before });
        }
    }

    canvas.dirty_rect = match canvas.dirty_rect {
        Some(rectangle) => Some(rectangle.union(&dirty_rect)),
        None => Some(dirty_rect),
    };

    UndoRecord::Run {
        layer_index: layer,
        color_after: color,
        runs,
        is_alpha_overlay: alpha_overlay,
    }
}
