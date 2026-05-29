//! Shared parameter bundle for brush-stroke line-drawing functions.
//!
//! [`BrushStrokeParams`] groups the common parameters that every
//! `draw_*_line` function accepts (start/end coordinates, canvas,
//! colour, layer, visited/drag stamps).  Tool-specific parameters
//! (radius, sampling, tinting, spacing, stamp pixels) remain as
//! individual arguments alongside this struct.

use eframe::egui::Color32;

use crate::canvas::Canvas;

/// Parameters common to all brush-stroke line-drawing functions.
///
/// Every `draw_*_line` function in `src/tools/` takes this bundle
/// plus tool-specific arguments, reducing visible boilerplate and
/// making signatures easier to read.
pub struct BrushStrokeParams<'a> {
    /// Column of the line start point.
    pub start_x: u32,
    /// Row of the line start point.
    pub start_y: u32,
    /// Column of the line end point.
    pub end_x: u32,
    /// Row of the line end point.
    pub end_y: u32,
    /// The canvas whose pixels will be modified.
    pub canvas: &'a mut Canvas,
    /// Stroke colour (premultiplied-alpha).
    pub color: Color32,
    /// Index of the target layer.
    pub layer: usize,
    /// Per-stroke stamp buffer for pixel deduplication.
    pub visited: &'a mut [u32],
    /// Current stroke-scoped stamp value.
    pub stamp: u32,
    /// Whether to alpha-blend instead of overwriting.
    pub alpha_overlay: bool,
    /// Per-drag-gesture deduplication buffer for alpha blend frames.
    pub drag_processed: &'a mut [u32],
    /// Current drag-scoped stamp value.
    pub drag_stamp_value: u32,
}
