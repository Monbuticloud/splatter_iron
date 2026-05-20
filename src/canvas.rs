use std::time::Duration;

use eframe::egui::{ self, Color32, TextureHandle };
use rayon::prelude::*;
use serde::{ Deserialize, Serialize };

use crate::pixel::{self, premultiply};

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
    #[serde(skip)]
    pub placeholder_texture: Option<TextureHandle>,
    #[serde(skip)]
    pub output_rgba: Vec<u8>,

    pub render_next_frame: bool,
}

impl Default for Canvas {
    fn default() -> Self {
        let layers: Vec<Layer> = vec![Layer {
            pixels: vec![Color32::TRANSPARENT; 12 * 1_000_000],
        }];
        Self {
            pixels: layers,
            height: 3000,
            width: 4000,
            output_rgba: Vec::new(),
            rendered_layers: None,
            placeholder_texture: None,
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
pub fn composite_layers_parallel_rgba(layers: &[Layer], output: &mut [u8]) {
    if layers.is_empty() {
        return;
    }
    output
        .par_chunks_mut(4)
        .enumerate()
        .for_each(|(i, out_px)| {
            let mut px = layers[0].pixels[i];
            for layer in &layers[1..] {
                px = pixel::alpha_blend(px, layer.pixels[i]);
            }
            out_px[0] = px.r();
            out_px[1] = px.g();
            out_px[2] = px.b();
            out_px[3] = px.a();
        });
}

/// Draw a filled rectangle on layer 0.
pub fn draw_square(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    canvas: &mut Canvas,
    color: Color32
) {
    let pm_color = premultiply(color);
    for x in start_x..end_x {
        for y in start_y..end_y {
            let idx = (x + y * canvas.width) as usize;
            if idx < canvas.pixels[0].pixels.len() {
                canvas.pixels[0].pixels[idx] = pm_color;
            }
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
