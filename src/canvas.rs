use std::time::Duration;

use eframe::egui::{ self, Color32, TextureHandle };
use serde::{ Deserialize, Serialize };

use crate::pixel::premultiply;
use crate::undo::{ self, Stroke, StrokePixel };

const DEFAULT_WIDTH: u32 = 2000;
const DEFAULT_HEIGHT: u32 = 1500;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub pixels: Vec<Color32>,
}

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

/// Internal: fill a rectangle without capturing undo data.
/// `pixels` is the mutable slice for the layer, `width` is canvas width.
#[inline(always)]
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

#[inline(always)]
pub fn draw_square(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    canvas: &mut Canvas,
    color: egui::Color32,
    layer: usize
) -> Stroke {
    let color = premultiply(color);
    let color_u32 = undo::color32_to_u32(color);

    let width = canvas.width as usize;
    let height = canvas.height;

    // Clamp once outside the hot loop
    let start_x = start_x.min(canvas.width);
    let end_x = end_x.min(canvas.width);
    let start_y = start_y.min(height);
    let end_y = end_y.min(height);

    // Early out
    if start_x >= end_x || start_y >= end_y {
        return Stroke {
            layer_index: layer,
            width: canvas.width,
            pixels: Vec::new(),
        };
    }

    let pixels = &mut canvas.pixels[layer].pixels;

    // Capture before colors and fill in one pass
    let mut stroke_pixels = Vec::with_capacity(((end_y - start_y) * (end_x - start_x)) as usize);

    for y in start_y..end_y {
        let row_start = (y as usize) * width;
        let start = row_start + (start_x as usize);
        let end = row_start + (end_x as usize);

        for i in start..end {
            let color_before = undo::color32_to_u32(pixels[i]);
            stroke_pixels.push(StrokePixel {
                index: i as u32,
                color_before,
                color_after: color_u32,
            });
        }
    }

    // Fill the rectangle (efficient contiguous write)
    fill_square_impl(pixels, width, start_x, end_x, start_y, end_y, color);

    Stroke {
        layer_index: layer,
        width: canvas.width,
        pixels: stroke_pixels,
    }
}

/// Collect all unique pixel indices touched by a brush line from (start_x, start_y) to (end_x, end_y).
/// Uses Bresenham line algorithm + visited-stamp dedup (no sort).
#[inline(always)]
fn collect_line_positions<'a>(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    brush_radius: u32,
    width: usize,
    height: u32,
    visited: &'a mut [u32],
    stamp: u32,
    bump: &'a bumpalo::Bump
) -> bumpalo::collections::Vec<'a, u32> {
    let half_radius = brush_radius >> 1;
    let mut positions = bumpalo::collections::Vec::new_in(bump);

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
                if visited[idx as usize] != stamp {
                    visited[idx as usize] = stamp;
                    positions.push(idx);
                }
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

    positions
}

#[inline(always)]
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
    bump_allocator: &bumpalo::Bump
) -> Stroke {
    let color = premultiply(color);
    let color_u32 = undo::color32_to_u32(color);
    let width = canvas.width as usize;
    let height = canvas.height;

    let positions = collect_line_positions(
        start_x,
        start_y,
        end_x,
        end_y,
        brush_radius,
        width,
        height,
        visited,
        stamp,
        bump_allocator
    );

    let pixels = &mut canvas.pixels[layer].pixels;

    // Single pass: snapshot before-color and write new color
    let mut stroke_pixels = Vec::with_capacity(positions.len());
    for &idx in &positions {
        let color_before = undo::color32_to_u32(pixels[idx as usize]);
        stroke_pixels.push(StrokePixel {
            index: idx,
            color_before,
            color_after: color_u32,
        });
        pixels[idx as usize] = color;
    }

    Stroke {
        layer_index: layer,
        width: canvas.width,
        pixels: stroke_pixels,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CurrentTool {
    SquareTool,
    CircleTool,
    SquareEraserTool,
    CircleEraserTool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RenderState {
    ActiveWake(Duration), // Full rendering
    IdleThrottled, // Slow repainting, frames still run but repainting is throttled
    UnfocusedFrozen, // No rendering
}
