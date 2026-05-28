//! Core canvas and layer types, brush-tool enum ([`CurrentTool`]),
//! render-state machine ([`RenderState`]), and dirty-rect tracking.

use std::time::Duration;

use eframe::egui::{ Color32, TextureHandle };
use serde::{ Deserialize, Serialize };

const DEFAULT_WIDTH: u32 = 2000;
const DEFAULT_HEIGHT: u32 = 1500;

/// A single layer of pixels in the canvas.
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub pixels: Vec<Color32>,
}

/// Axis-aligned bounding box of a modified region on the canvas.
#[derive(Clone, Copy, Default)]
pub struct DirtyRect {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

impl DirtyRect {
    /// Create a `DirtyRect` with given bounds.
    pub const fn new(min_x: u32, min_y: u32, max_x: u32, max_y: u32) -> Self {
        Self { min_x, min_y, max_x, max_y }
    }

    /// Create an empty rect that expands on the first `extend` call.
    pub const fn empty() -> Self {
        Self { min_x: u32::MAX, min_y: u32::MAX, max_x: 0, max_y: 0 }
    }

    /// Expand the rect to include `(x, y)`.
    pub fn extend(&mut self, x: u32, y: u32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    /// Merge with another rect.
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    /// Returns `true` when the rect covers no pixels (either dimension is inverted).
    pub const fn is_empty(&self) -> bool {
        self.min_x > self.max_x || self.min_y > self.max_y
    }

    /// Number of columns in the rect, or `0` if empty.
    pub const fn width(&self) -> u32 {
        if self.is_empty() { 0 } else { self.max_x - self.min_x + 1 }
    }

    /// Number of rows in the rect, or `0` if empty.
    pub const fn height(&self) -> u32 {
        if self.is_empty() { 0 } else { self.max_y - self.min_y + 1 }
    }
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

    /// Bounding box of pixels that changed since the last texture upload.
    /// `None` means a full re-blend is needed (e.g. after layer reorder).
    #[serde(skip)]
    pub dirty_rect: Option<DirtyRect>,

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
            dirty_rect: None,
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
            dirty_rect: None,
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
    BucketFill,
}

/// Desired rendering cadence: active wake for fast repaints,
/// idle throttled for slow repaints, or frozen when viewport is unfocused.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RenderState {
    ActiveWake(Duration), // Full rendering
    IdleThrottled, // Slow repainting, frames still run but repainting is throttled
    UnfocusedFrozen, // No rendering
}
