//! Core canvas and layer types, brush-tool enum ([`CurrentTool`]),
//! render-state machine ([`RenderState`]), and dirty-rect tracking.

use std::time::Duration;

use eframe::egui::Color32;
use eframe::egui::TextureHandle;
use serde::Deserialize;
use serde::Serialize;

const DEFAULT_WIDTH: u32 = 2000;
const DEFAULT_HEIGHT: u32 = 1500;

/// A single layer of pixels in the canvas.
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Layer {
    /// Premultiplied-alpha RGBA pixels in row-major order.
    pub pixels: Vec<Color32>,
}

/// Axis-aligned bounding box of a modified region on the canvas.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct DirtyRect {
    /// Minimum column index (inclusive) of the dirty region.
    pub min_x: u32,
    /// Minimum row index (inclusive) of the dirty region.
    pub min_y: u32,
    /// Maximum column index (inclusive) of the dirty region.
    pub max_x: u32,
    /// Maximum row index (inclusive) of the dirty region.
    pub max_y: u32,
}

impl DirtyRect {
    /// Create a `DirtyRect` with given bounds.
    pub const fn new(min_x: u32, min_y: u32, max_x: u32, max_y: u32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    /// Create an empty rect that expands on the first `extend` call.
    pub const fn empty() -> Self {
        Self {
            min_x: u32::MAX,
            min_y: u32::MAX,
            max_x: 0,
            max_y: 0,
        }
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
        if self.is_empty() {
            0
        } else {
            self.max_x - self.min_x + 1
        }
    }

    /// Number of rows in the rect, or `0` if empty.
    pub const fn height(&self) -> u32 {
        if self.is_empty() {
            0
        } else {
            self.max_y - self.min_y + 1
        }
    }
}

/// Minimum gap between two dirty rects before they are merged (in pixels).
const DIRTY_RECT_PROXIMITY: u32 = 16;

/// Maximum number of dirty rects before merging all into one bounding box.
const DIRTY_RECT_MAX_COUNT: usize = 8;

/// A list of dirty rectangles that tracks each dirty region individually,
/// merging overlapping or proximate rects. When the count exceeds
/// [`DIRTY_RECT_MAX_COUNT`], all rects are merged into a single bounding box.
#[derive(Clone, Default)]
pub struct DirtyRectList {
    rects: Vec<DirtyRect>,
}

impl DirtyRectList {
    /// Create an empty list.
    pub fn new() -> Self {
        Self { rects: Vec::new() }
    }

    /// Add a dirty rect, merging with overlapping or proximate rects.
    ///
    /// Two rects are merged when they overlap or when the gap between them
    /// is ≤ [`DIRTY_RECT_PROXIMITY`] pixels. If the total exceeds
    /// [`DIRTY_RECT_MAX_COUNT`], all rects are merged into one.
    pub fn add(&mut self, rect: DirtyRect) {
        if rect.is_empty() {
            return;
        }

        let mut merged = rect;
        let mut i = 0;
        while i < self.rects.len() {
            if rects_overlap_or_touch(&self.rects[i], &merged, DIRTY_RECT_PROXIMITY) {
                merged = merged.union(&self.rects[i]);
                self.rects.swap_remove(i);
            } else {
                i += 1;
            }
        }

        self.rects.push(merged);

        if self.rects.len() > DIRTY_RECT_MAX_COUNT {
            self.merge_all();
        }
    }

    /// Merge all tracked rects into a single bounding box.
    pub fn merge_all(&mut self) {
        if self.rects.is_empty() {
            return;
        }
        let mut unified = DirtyRect::empty();
        for rect in &self.rects {
            unified = unified.union(rect);
        }
        self.rects.clear();
        self.rects.push(unified);
    }

    /// Take all tracked rects and reset the list to empty.
    pub fn take_all(&mut self) -> Vec<DirtyRect> {
        std::mem::take(&mut self.rects)
    }

    /// Returns `true` when no rects are tracked.
    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }

    /// Clear all tracked rects.
    pub fn clear(&mut self) {
        self.rects.clear();
    }
}

/// Returns `true` if two dirty rects overlap or are within `proximity` pixels.
fn rects_overlap_or_touch(a: &DirtyRect, b: &DirtyRect, proximity: u32) -> bool {
    let a_min_x = a.min_x.saturating_sub(proximity);
    let a_min_y = a.min_y.saturating_sub(proximity);
    let a_max_x = a.max_x.saturating_add(proximity);
    let a_max_y = a.max_y.saturating_add(proximity);

    !(a_max_x < b.min_x || a_min_x > b.max_x || a_max_y < b.min_y || a_min_y > b.max_y)
}

/// The core canvas data: a list of layers, dimensions, output RGBA buffer,
/// GPU texture state, and a flag to request re-rendering.
#[derive(Clone, Serialize, Deserialize)]
pub struct Canvas {
    /// Ordered list of layers from bottom (index 0) to top.
    pub pixels: Vec<Layer>,
    /// Canvas height in pixels.
    pub height: u32,
    /// Canvas width in pixels.
    pub width: u32,
    /// Cached GPU texture handle for rendered composite (egui-managed).
    #[serde(skip)]
    pub rendered_layers: Option<TextureHandle>,
    /// Premultiplied-alpha output buffer holding the blended result
    /// of all layers (width × height × 4 bytes).
    #[serde(skip)]
    pub output_rgba: Vec<u8>,

    /// Regions that changed since the last texture upload.
    /// When empty, a full re-blend is needed (e.g. after layer reorder).
    #[serde(skip)]
    pub dirty_rect: DirtyRectList,

    /// Flag to request a full re-render on the next frame.
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
            dirty_rect: DirtyRectList::new(),
            render_next_frame: true,
        }
    }
}

impl Canvas {
    /// Create a new canvas with the given dimensions and a single transparent layer.
    ///
    /// # Parameters
    ///
    /// * `width` — Canvas width in pixels.
    /// * `height` — Canvas height in pixels.
    ///
    /// # Panics
    ///
    /// Panics if `width * height` overflows `usize` (extremely unlikely in practice).
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width as usize)
            .checked_mul(height as usize)
            .expect("Canvas dimensions overflow usize");
        Self {
            pixels: vec![Layer {
                pixels: vec![Color32::TRANSPARENT; pixel_count],
            }],
            width,
            height,
            output_rgba: Vec::new(),
            rendered_layers: None,
            dirty_rect: DirtyRectList::new(),
            render_next_frame: true,
        }
    }
}

/// The drawing tool currently selected in the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurrentTool {
    /// Draw solid rectangles by dragging.
    Square,
    /// Draw solid circles by dragging.
    Circle,
    /// Erase by dragging a rectangular eraser.
    SquareEraser,
    /// Erase by dragging a circular eraser.
    CircleEraser,
    /// Flood-fill a contiguous region of similar color.
    BucketFill,
    /// Stamp an external image onto the canvas.
    Stamp,
    /// Draw using a custom brush tip from the brush library.
    CustomBrush,
}

/// Desired rendering cadence: active wake for fast repaints,
/// idle throttled for slow repaints, or frozen when viewport is unfocused.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RenderState {
    /// Full rendering — canvas redraws every frame (active interaction).
    ActiveWake(Duration),
    /// Slow repainting — frames still run but canvas repainting is throttled.
    IdleThrottled,
    /// No rendering — viewport is unfocused, all GPU work suspended.
    UnfocusedFrozen,
}
