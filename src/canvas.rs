use std::time::Duration;

use eframe::egui::{ self, Color32, TextureHandle };
use serde::{ Deserialize, Serialize };

use crate::pixel::premultiply;
use crate::undo::{ self, Stroke, StrokePixel };

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
        let layers: Vec<Layer> = vec![Layer {
            pixels: vec![Color32::TRANSPARENT; 3 * 1_000_000],
        }];
        Self {
            pixels: layers,
            height: 1500,
            width: 2000,
            output_rgba: Vec::new(),
            rendered_layers: None,
            // placeholder_texture: None,
            render_next_frame: true,
        }
    }
}

// /// Composite layers into RGBA byte buffer using SIMD-accelerated blending.
// /// Replaced by direct pixel::blend_layers call in app.rs.
// #[inline(always)]
// pub fn composite_layers_parallel_rgba(layers: &[Layer], output: &mut [u8]) {
//     if layers.is_empty() {
//         return;
//     }
//     let layer_slices: Vec<&[Color32]> = layers.iter().map(|l| l.pixels.as_slice()).collect();
//     pixel::blend_layers(&layer_slices, output);
// }

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
    for y in start_y..end_y {
        let row_start = (y as usize) * width;
        let start = row_start + (start_x as usize);
        let end = row_start + (end_x as usize);
        pixels[start..end].fill(color);
    }

    Stroke {
        layer_index: layer,
        width: canvas.width,
        pixels: stroke_pixels,
    }
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
    layer: usize
) -> Stroke {
    let color = premultiply(color);
    let color_u32 = undo::color32_to_u32(color);

    let half_radius = brush_radius >> 1; // brush_radius / 2
    let width = canvas.width as usize;
    let height = canvas.height;

    // First pass: collect all unique pixel positions touched by the line
    let mut positions = Vec::new();

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

        for y in brush_start_y..brush_end_y {
            let row_start = (y as usize) * width;
            for x in brush_start_x..brush_end_x {
                positions.push((row_start + (x as usize)) as u32);
            }
        }

        if current_x == target_x && current_y == target_y {
            break;
        }
        let error_doubled = error.saturating_mul(2);
        if error_doubled >= delta_y {
            error += delta_y;
            current_x += step_x;
        }
        if error_doubled <= delta_x {
            error += delta_x;
            current_y += step_y;
        }
    }

    // Sort and deduplicate
    positions.sort_unstable();
    positions.dedup();

    let pixels = &mut canvas.pixels[layer].pixels;

    // Capture before colors
    let mut stroke_pixels = Vec::with_capacity(positions.len());
    for &idx in &positions {
        let color_before = undo::color32_to_u32(pixels[idx as usize]);
        stroke_pixels.push(StrokePixel {
            index: idx,
            color_before,
            color_after: color_u32,
        });
    }

    // Second pass: draw the line
    let mut current_x = start_x as i32;
    let mut current_y = start_y as i32;
    let sp_delta_x = target_x.abs_diff(start_x as i32) as i32;
    let sp_delta_y = -(target_y.abs_diff(start_y as i32) as i32);
    let mut error = sp_delta_x + sp_delta_y;

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

        fill_square_impl(
            pixels,
            width,
            brush_start_x,
            brush_end_x,
            brush_start_y,
            brush_end_y,
            color
        );

        if current_x == target_x && current_y == target_y {
            break;
        }
        let error_doubled = error.saturating_mul(2);
        if error_doubled >= sp_delta_y {
            error += sp_delta_y;
            current_x += step_x;
        }
        if error_doubled <= sp_delta_x {
            error += sp_delta_x;
            current_y += step_y;
        }
    }

    Stroke {
        layer_index: layer,
        width: canvas.width,
        pixels: stroke_pixels,
    }
}

#[inline(always)]
pub fn erase_square_line(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    brush_radius: u32,
    canvas: &mut Canvas,
    layer: usize
) -> Stroke {
    let transparent = Color32::TRANSPARENT;
    let transparent_u32 = undo::color32_to_u32(transparent);

    let half_radius = brush_radius >> 1; // brush_radius / 2
    let width = canvas.width as usize;
    let height = canvas.height;

    // First pass: collect all unique pixel positions touched by the line
    let mut positions = Vec::new();

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

        for y in brush_start_y..brush_end_y {
            let row_start = (y as usize) * width;
            for x in brush_start_x..brush_end_x {
                positions.push((row_start + (x as usize)) as u32);
            }
        }

        if current_x == target_x && current_y == target_y {
            break;
        }
        let error_doubled = error.saturating_mul(2);
        if error_doubled >= delta_y {
            error += delta_y;
            current_x += step_x;
        }
        if error_doubled <= delta_x {
            error += delta_x;
            current_y += step_y;
        }
    }

    // Sort and deduplicate
    positions.sort_unstable();
    positions.dedup();

    let pixels = &mut canvas.pixels[layer].pixels;

    // Capture before colors
    let mut stroke_pixels = Vec::with_capacity(positions.len());
    for &idx in &positions {
        let color_before = undo::color32_to_u32(pixels[idx as usize]);
        stroke_pixels.push(StrokePixel {
            index: idx,
            color_before,
            color_after: transparent_u32,
        });
    }

    // Second pass: draw the line with TRANSPARENT
    let mut current_x = start_x as i32;
    let mut current_y = start_y as i32;
    let sp_delta_x = target_x.abs_diff(start_x as i32) as i32;
    let sp_delta_y = -(target_y.abs_diff(start_y as i32) as i32);
    let mut error = sp_delta_x + sp_delta_y;

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

        fill_square_impl(
            pixels,
            width,
            brush_start_x,
            brush_end_x,
            brush_start_y,
            brush_end_y,
            transparent
        );

        if current_x == target_x && current_y == target_y {
            break;
        }
        let error_doubled = error.saturating_mul(2);
        if error_doubled >= sp_delta_y {
            error += sp_delta_y;
            current_x += step_x;
        }
        if error_doubled <= sp_delta_x {
            error += sp_delta_x;
            current_y += step_y;
        }
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
    Warm(Duration),
    Cold,
    Frozen,
}
