use std::time::Duration;

use eframe::egui::{ Color32, TextureHandle };
use serde::{ Deserialize, Serialize };

pub use crate::tools::circle_brush::{ draw_circle, draw_circle_line };
pub use crate::tools::square_brush::{ draw_square, draw_square_line };

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

impl Canvas {
    /// Create a new canvas with the given dimensions and a single transparent layer.
    ///
    /// # Panics
    ///
    /// Panics if `width * height` overflows `usize` (extremely unlikely in practice).
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width as usize).checked_mul(height as usize).expect(
            "Canvas dimensions overflow usize"
        );
        Self {
            pixels: vec![Layer { pixels: vec![Color32::TRANSPARENT; pixel_count] }],
            width,
            height,
            output_rgba: Vec::new(),
            rendered_layers: None,
            render_next_frame: true,
        }
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
