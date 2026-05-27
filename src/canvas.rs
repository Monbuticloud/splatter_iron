use std::time::Duration;

use eframe::egui::{ self, Color32, TextureHandle };
use serde::{ Deserialize, Serialize };

use crate::pixel::premultiply;
use crate::undo::{ UndoRecord, RunSegment, compress_run };

const DEFAULT_WIDTH: u32 = 2000;
const DEFAULT_HEIGHT: u32 = 1500;

/// A single layer of pixels in the canvas.
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub pixels: Vec<Color32>,
}

/// The core canvas data: a list of layers, dimensions, output RGBA buffer,
/// GPU texture state, and a flag to request re-rendering.
#[derive(Clone, Serialize, Deserialize)]
pub struct Canvas {
    pub pixels: Vec<Layer>,
    pub height: u32,
    pub width: u32,
    #[serde(skip)]
    pub rendered_layers: Option<TextureHandle>,
    // #[serde(skip)]
    // pub placeholder_texture: Option<TextureHandle>,
    #[serde(skip)]
    pub output_rgba: Vec<u8>,

    pub render_next_frame: bool,
}

impl Default for Canvas {
    /// Create a default 2000×1500 canvas with a single transparent layer.
    fn default() -> Self {
        let pixel_count = (DEFAULT_WIDTH * DEFAULT_HEIGHT) as usize;
        let layers: Vec<Layer> = vec![Layer {
            pixels: vec![Color32::TRANSPARENT; pixel_count],
        }];
        Self {
            pixels: layers,
            height: DEFAULT_HEIGHT,
            width: DEFAULT_WIDTH,
            output_rgba: Vec::new(),
            rendered_layers: None,
            // placeholder_texture: None,
            render_next_frame: true,
        }
    }
}

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
    color: Color32
) {
    for y in start_y..end_y {
        let row_start = (y as usize) * width;
        let start = row_start + (start_x as usize);
        let end = row_start + (end_x as usize);
        pixels[start..end].fill(color);
    }
}

/// Draw a filled rectangle on a canvas layer and return an undo record.
///
/// Coordinates are clamped to canvas bounds. If `start_x >= end_x` or
/// `start_y >= end_y` the call is a no-op and returns an empty undo record.
/// Captures before-pixel data for every touched position to support undo.
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
    layer: usize
) -> UndoRecord {
    let color = premultiply(color);

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
            width: canvas.width,
            color_after: color,
            runs: Vec::new(),
        };
    }

    let pixels = &mut canvas.pixels[layer].pixels;

    // Capture runs (one per row) and fill in one pass
    let mut runs = Vec::with_capacity((end_y - start_y) as usize);

    for y in start_y..end_y {
        let row_start = (y as usize) * width;
        let start = row_start + (start_x as usize);
        let end = row_start + (end_x as usize);
        let run_len = end - start;

        let mut before = Vec::with_capacity(run_len);
        before.extend_from_slice(&pixels[start..end]);
        let (before, len) = compress_run(before);

        runs.push(RunSegment {
            start: start as u32,
            len,
            before,
        });
    }

    // Fill the rectangle (efficient contiguous write)
    fill_square_impl(pixels, width, start_x, end_x, start_y, end_y, color);

    UndoRecord::Run {
        layer_index: layer,
        width: canvas.width,
        color_after: color,
        runs,
    }
}

/// Mark all pixel indices covered by a brush stroke line in the `visited` buffer.
///
/// Uses the Bresenham line algorithm to step along the line and stamps every
/// pixel within the brush radius (a square brush). The caller can later scan
/// `visited` for values matching `stamp` to get deduplicated, sorted positions.
///
/// Clamps brush bounds to canvas dimensions.
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
) {
    let half_radius = brush_radius;

    let mut current_x = start_x as i32;
    let mut current_y = start_y as i32;
    let target_x = end_x as i32;
    let target_y = end_y as i32;

    let delta_x = target_x.abs_diff(current_x) as i32;
    let step_x = if current_x < target_x { 1 } else { -1 };
    let delta_y = -(target_y.abs_diff(current_y) as i32);
    let step_y = if current_y < target_y { 1 } else { -1 };
    let mut err = delta_x + delta_y;

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

        for y in brush_start_y..brush_end_y {
            let row_start = (y as usize) * width;
            for x in brush_start_x..brush_end_x {
                let idx = (row_start + (x as usize)) as u32;
                visited[idx as usize] = stamp;
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

/// Draw a brush line between two points and return an undo record.
///
/// Uses `stamp_line_positions` to find all touched pixels, then applies the
/// color and captures before-data for undo. The `visited` buffer and `stamp`
/// value must be managed by the caller to avoid re-processing old stamps.
///
/// # Panics
///
/// Panics if `layer >= canvas.pixels.len()`.
#[inline]
pub fn draw_square_line(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    brush_radius: u32,
    canvas: &mut Canvas,
    color: egui::Color32,
    layer: usize,
    visited: &mut [u32],
    stamp: u32,
) -> UndoRecord {
    let color = premultiply(color);
    let width = canvas.width as usize;
    let height = canvas.height;
    let pixel_count = (width as u32) * height;

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
    );

    let pixels = &mut canvas.pixels[layer].pixels;

    // Scan visited to build runs in sorted order
    let mut runs: Vec<RunSegment> = Vec::new();
    let mut i = 0u32;
    while i < pixel_count {
        if visited[i as usize] != stamp {
            i += 1;
            continue;
        }
        let start = i;
        let mut before = Vec::new();
        while i < pixel_count && visited[i as usize] == stamp {
            before.push(pixels[i as usize]);
            pixels[i as usize] = color;
            i += 1;
        }
        let (rle_before, len) = compress_run(before);
        runs.push(RunSegment { start, len, before: rle_before });
    }

    UndoRecord::Run {
        layer_index: layer,
        width: canvas.width,
        color_after: color,
        runs,
    }
}

// ---------------------------------------------------------------------------
// Circle brush primitives
// ---------------------------------------------------------------------------

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
) {
    if radius == 0 {
        let idx = (center_y as usize) * width + center_x as usize;
        pixels[idx] = color;
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

        // Top half row
        if let Some(y) = center_y.checked_sub(dy) {
            let row_start = (y as usize) * width;
            pixels[row_start + span_start as usize..=row_start + span_end as usize].fill(color);
        }

        // Bottom half (skip centre-row duplicate)
        if dy != 0 {
            let y = center_y + dy;
            if y < canvas_height {
                let row_start = (y as usize) * width;
                pixels[row_start + span_start as usize..=row_start + span_end as usize].fill(color);
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
) -> UndoRecord {
    let color = premultiply(color);

    let width = canvas.width as usize;
    let height = canvas.height;

    // Clamp center to canvas
    let center_x = center_x.min(canvas.width);
    let center_y = center_y.min(height);

    if radius == 0 || center_x >= canvas.width || center_y >= height {
        return UndoRecord::Run {
            layer_index: layer,
            width: canvas.width,
            color_after: color,
            runs: Vec::new(),
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
            let (before, len) = compress_run(before);
            runs.push(RunSegment {
                start: s as u32,
                len,
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
                let (before, len) = compress_run(before);
                runs.push(RunSegment {
                    start: s as u32,
                    len,
                    before,
                });
            }
        }
    }

    // Fill the circle
    fill_circle_impl(pixels, width, center_x, center_y, radius, color, canvas.width, height);

    UndoRecord::Run {
        layer_index: layer,
        width: canvas.width,
        color_after: color,
        runs,
    }
}

/// Mark all pixel indices covered by a circle-brush stroke line in the `visited` buffer.
///
/// Uses the Bresenham line algorithm to step along the line and stamps every
/// pixel within the circle radius (geometric radius) at each step.
/// The caller can later scan `visited` for values matching `stamp` to get
/// deduplicated, sorted positions.
///
/// Clamps brush bounds to canvas dimensions.
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
                visited[(cy as usize) * width + cx as usize] = stamp;
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
        for dy in -(geo_radius as i32)..=(geo_radius as i32) {
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
) -> UndoRecord {
    let color = premultiply(color);
    let width = canvas.width as usize;
    let height = canvas.height;
    let pixel_count = (width as u32) * height;

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
    );

    let pixels = &mut canvas.pixels[layer].pixels;

    let mut runs: Vec<RunSegment> = Vec::new();
    let mut i = 0u32;
    while i < pixel_count {
        if visited[i as usize] != stamp {
            i += 1;
            continue;
        }
        let start = i;
        let mut before = Vec::new();
        while i < pixel_count && visited[i as usize] == stamp {
            before.push(pixels[i as usize]);
            pixels[i as usize] = color;
            i += 1;
        }
        let (rle_before, len) = compress_run(before);
        runs.push(RunSegment { start, len, before: rle_before });
    }

    UndoRecord::Run {
        layer_index: layer,
        width: canvas.width,
        color_after: color,
        runs,
    }
}

/// The drawing tool currently selected in the UI.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CurrentTool {
    Square,
    Circle,
    SquareEraser,
    CircleEraser,
}

/// Desired rendering cadence: active wake for fast repaints,
/// idle throttled for slow repaints, or frozen when viewport is unfocused.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RenderState {
    ActiveWake(Duration), // Full rendering
    IdleThrottled, // Slow repainting, frames still run but repainting is throttled
    UnfocusedFrozen, // No rendering
}
