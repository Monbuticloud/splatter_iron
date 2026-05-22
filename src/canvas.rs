use std::time::Duration;

use eframe::egui::{ self, Color32, TextureHandle };
use rayon::prelude::*;
use serde::{ Deserialize, Serialize };
// use crate::undo::*;

use crate::pixel::{ self, premultiply };

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

/// Composite all layers sequentially (single-threaded).
// pub fn composite_all_layers(layers: &[Vec<Color32>], output: &mut Vec<Color32>) {
//     if layers.is_empty() {
//         return;
//     }
//     output.copy_from_slice(&layers[0]);
//     for layer in &layers[1..] {
//         for i in 0..output.len() {
//             output[i] = pixel::alpha_blend(output[i], layer[i]);
//         }
//     }
// }
/// Unused

/// Composite layers into RGBA byte buffer using parallel iteration.
#[inline(always)]
pub fn composite_layers_parallel_rgba(layers: &[Layer], output: &mut [u8]) {
    if layers.is_empty() {
        return;
    }
    output
        .par_chunks_mut(4)
        .enumerate()
        .for_each(|(i, output_pixel)| {
            let mut pixel = layers[0].pixels[i];
            for layer in &layers[1..] {
                pixel = pixel::alpha_blend(pixel, layer.pixels[i]);
            }
            output_pixel[0] = pixel.r();
            output_pixel[1] = pixel.g();
            output_pixel[2] = pixel.b();
            output_pixel[3] = pixel.a();
        });
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
) {
    let color = premultiply(color);

    let width = canvas.width as usize;
    let height = canvas.height;
    let pixels = &mut canvas.pixels[layer].pixels;

    // Clamp once outside the hot loop
    let start_x = start_x.min(canvas.width);
    let end_x = end_x.min(canvas.width);

    let start_y = start_y.min(height);
    let end_y = end_y.min(height);

    // Early out
    if start_x >= end_x || start_y >= end_y {
        return;
    }

    // Row-major traversal (cache friendly)
    for y in start_y..end_y {
        let row_start = (y as usize) * width;

        let start = row_start + (start_x as usize);
        let end = row_start + (end_x as usize);

        // Extremely optimized contiguous write
        pixels[start..end].fill(color);
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
) {
    let half_radius = brush_radius / 2;

    let mut current_x = start_x as i32;
    let mut current_y = start_y as i32;

    let target_x = end_x as i32;
    let target_y = end_y as i32;

    let delta_x = (target_x - current_x).abs();
    let step_x = if current_x < target_x { 1 } else { -1 };

    let delta_y = -(target_y - current_y).abs();
    let step_y = if current_y < target_y { 1 } else { -1 };

    let mut error = delta_x + delta_y;

    loop {
        let brush_start_x = current_x.saturating_sub(half_radius as i32) as u32;
        let brush_end_x = (current_x + (half_radius as i32) + 1).min(canvas.width as i32) as u32;

        let brush_start_y = current_y.saturating_sub(half_radius as i32) as u32;
        let brush_end_y = (current_y + (half_radius as i32) + 1).min(canvas.height as i32) as u32;

        draw_square(brush_start_x, brush_start_y, brush_end_x, brush_end_y, canvas, color, layer);

        if current_x == target_x && current_y == target_y {
            break;
        }

        let error_doubled = error << 1;

        if error_doubled >= delta_y {
            error += delta_y;
            current_x += step_x;
        }

        if error_doubled <= delta_x {
            error += delta_x;
            current_y += step_y;
        }
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
) {
    let half_radius = brush_radius / 2;

    let mut current_x = start_x as i32;
    let mut current_y = start_y as i32;

    let target_x = end_x as i32;
    let target_y = end_y as i32;

    let delta_x = (target_x - current_x).abs();
    let step_x = if current_x < target_x { 1 } else { -1 };

    let delta_y = -(target_y - current_y).abs();
    let step_y = if current_y < target_y { 1 } else { -1 };

    let mut error = delta_x + delta_y;

    loop {
        let brush_start_x = current_x.saturating_sub(half_radius as i32) as u32;
        let brush_end_x = (current_x + (half_radius as i32) + 1).min(canvas.width as i32) as u32;

        let brush_start_y = current_y.saturating_sub(half_radius as i32) as u32;
        let brush_end_y = (current_y + (half_radius as i32) + 1).min(canvas.height as i32) as u32;

        draw_square(
            brush_start_x,
            brush_start_y,
            brush_end_x,
            brush_end_y,
            canvas,
            Color32::TRANSPARENT,
            layer
        );

        if current_x == target_x && current_y == target_y {
            break;
        }

        let error_doubled = error << 1;

        if error_doubled >= delta_y {
            error += delta_y;
            current_x += step_x;
        }

        if error_doubled <= delta_x {
            error += delta_x;
            current_y += step_y;
        }
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
