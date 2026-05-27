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

/// The core canvas data: a list of layers, dimensions, and GPU texture state.
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
    let half_radius = brush_radius >> 1;

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

/// The drawing tool currently selected in the UI.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CurrentTool {
    Square,
    Circle,
    SquareEraser,
    CircleEraser,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RenderState {
    ActiveWake(Duration), // Full rendering
    IdleThrottled, // Slow repainting, frames still run but repainting is throttled
    UnfocusedFrozen, // No rendering
}
