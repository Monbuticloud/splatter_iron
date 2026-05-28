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
        let idx = (center_y as usize) * width + center_x as usize;
        if alpha_overlay {
            pixels[idx] = crate::pixel::alpha_blend(pixels[idx], color);
        } else {
            pixels[idx] = color;
        }
        return;
    }

    let r_sq = (radius as u64) * (radius as u64);

    for dy in 0..=radius {
        let dy_sq = (dy as u64) * (dy as u64);
        let dx = ((r_sq - dy_sq) as f64).sqrt() as u32;
        let span_start = center_x.saturating_sub(dx).min(canvas_width - 1);
        let span_end = (center_x + dx).min(canvas_width - 1);
        if span_start > span_end {
            continue;
        }

        let apply = |span: &mut [Color32]| {
            if alpha_overlay {
                for p in span.iter_mut() {
                    *p = crate::pixel::alpha_blend(*p, color);
                }
            } else {
                span.fill(color);
            }
        };

        // Top half row
        if let Some(y) = center_y.checked_sub(dy) {
            let row_start = (y as usize) * width;
            apply(&mut pixels[row_start + span_start as usize..=row_start + span_end as usize]);
        }

        // Bottom half (skip centre-row duplicate)
        if dy != 0 {
            let y = center_y + dy;
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

    let r_sq = (radius as u64) * (radius as u64);

    for dy in 0..=radius {
        let dy_sq = (dy as u64) * (dy as u64);
        let dx = ((r_sq - dy_sq) as f64).sqrt() as u32;
        let span_start = center_x.saturating_sub(dx).min(canvas.width - 1);
        let span_end = (center_x + dx).min(canvas.width - 1);

        if span_start > span_end {
            continue;
        }

        // Top half row
        if let Some(y) = center_y.checked_sub(dy) {
            let row_start = (y as usize) * width;
            let s = row_start + span_start as usize;
            let e = row_start + span_end as usize + 1;
            let mut before = Vec::with_capacity(e - s);
            before.extend_from_slice(&pixels[s..e]);
            let (before, length) = compress_run(before);
            runs.push(RunSegment {
                start: s as u32,
                length,
                before,
            });
        }

        // Bottom half (skip centre-row duplicate)
        if dy != 0 {
            let y = center_y + dy;
            if y < height {
                let row_start = (y as usize) * width;
                let s = row_start + span_start as usize;
                let e = row_start + span_end as usize + 1;
                let mut before = Vec::with_capacity(e - s);
                before.extend_from_slice(&pixels[s..e]);
                let (before, length) = compress_run(before);
                runs.push(RunSegment {
                    start: s as u32,
                    length,
                    before,
                });
            }
        }
    }

    // Fill the circle
    fill_circle_impl(pixels, width, center_x, center_y, radius, color, canvas.width, height, alpha_overlay);

    let cx_min = center_x.saturating_sub(radius);
    let cy_min = center_y.saturating_sub(radius);
    let cx_max = (center_x + radius).min(canvas.width - 1);
    let cy_max = (center_y + radius).min(canvas.height - 1);
    let rect = DirtyRect::new(cx_min, cy_min, cx_max, cy_max);
    canvas.dirty_rect = match canvas.dirty_rect {
        Some(r) => Some(r.union(&rect)),
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
        let mut cx = start_x as i32;
        let mut cy = start_y as i32;
        let tx = end_x as i32;
        let ty = end_y as i32;
        let dx = tx.abs_diff(cx) as i32;
        let sx = if cx < tx { 1 } else { -1 };
        let dy = -(ty.abs_diff(cy) as i32);
        let sy = if cy < ty { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            if cx >= 0 && (cx as u32) < width as u32 && cy >= 0 && (cy as u32) < height {
                let x = cx as u32;
                let y = cy as u32;
                visited[(y as usize) * width + x as usize] = stamp;
                dirty_rect.extend(x, y);
            }
            if cx == tx && cy == ty {
                break;
            }
            let e2 = err.saturating_mul(2);
            if e2 >= dy {
                err += dy;
                cx += sx;
            }
            if e2 <= dx {
                err += dx;
                cy += sy;
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
    let mut err = delta_x + delta_y;

    let r_sq = (geo_radius as u64) * (geo_radius as u64);

    loop {
        let g = geo_radius as i32;
        let cy_min = (current_y - g).max(0) as u32;
        let cy_max = (current_y + g).min(height as i32 - 1).max(0) as u32;
        let cx_min = (current_x - g).max(0) as u32;
        let cx_max = (current_x + g).min(width as i32 - 1).max(0) as u32;
        dirty_rect.extend(cx_min, cy_min);
        dirty_rect.extend(cx_max, cy_max);

        for dy in -g..=g {
            let y = current_y + dy;
            if y < 0 || y >= height as i32 {
                continue;
            }
            let dy_sq = (dy.abs() as u64) * (dy.abs() as u64);
            let dx = ((r_sq - dy_sq) as f64).sqrt() as i32;
            let row_start = (y as usize) * width;
            let start_x_local = (current_x - dx).max(0).min(width as i32 - 1) as usize;
            let end_x_local = (current_x + dx).max(0).min(width as i32 - 1) as usize;
            for x in start_x_local..=end_x_local {
                visited[row_start + x] = stamp;
            }
        }

        if current_x == target_x && current_y == target_y {
            break;
        }
        let e2 = err.saturating_mul(2);
        if e2 >= delta_y {
            err += delta_y;
            current_x += step_x;
        }
        if e2 <= delta_x {
            err += delta_x;
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
    drag_stamp_val: u32,
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
            let idx = row_start + x as usize;
            if visited[idx] != stamp {
                x += 1;
                continue;
            }
            if alpha_overlay && drag_processed[idx] == drag_stamp_val {
                x += 1;
                continue;
            }
            let run_start = idx as u32;
            let mut before = Vec::new();
            while x <= dirty_rect.max_x {
                let idx2 = row_start + x as usize;
                if visited[idx2] != stamp {
                    break;
                }
                if alpha_overlay && drag_processed[idx2] == drag_stamp_val {
                    break;
                }
                before.push(pixels[idx2]);
                pixels[idx2] = if alpha_overlay {
                    crate::pixel::alpha_blend(pixels[idx2], color)
                } else {
                    color
                };
                if alpha_overlay {
                    drag_processed[idx2] = drag_stamp_val;
                }
                x += 1;
            }
            let (rle_before, length) = compress_run(before);
            runs.push(RunSegment { start: run_start, length, before: rle_before });
        }
    }

    canvas.dirty_rect = match canvas.dirty_rect {
        Some(r) => Some(r.union(&dirty_rect)),
        None => Some(dirty_rect),
    };

    UndoRecord::Run {
        layer_index: layer,
        color_after: color,
        runs,
        is_alpha_overlay: alpha_overlay,
    }
}
