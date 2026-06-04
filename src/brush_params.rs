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
///
/// Construct instances via [`BrushStrokeParams::builder`].

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

/// Builder for [`BrushStrokeParams`].
///
/// Captures the invariant fields (canvas, color, layer, scratch buffers)
/// once, then lets callers override only start/end position and
/// alpha_overlay per line segment.

pub(crate) struct BrushStrokeParamsBuilder<'a> {
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    canvas: &'a mut Canvas,
    color: Color32,
    layer: usize,
    visited: &'a mut [u32],
    stamp: u32,
    alpha_overlay: bool,
    drag_processed: &'a mut [u32],
    drag_stamp_value: u32,
}

impl<'a> BrushStrokeParams<'a> {
    /// Start building with the stroke-invariant fields.

    pub(crate) fn builder(
        canvas: &'a mut Canvas,
        color: Color32,
        layer: usize,
        visited: &'a mut [u32],
        stamp: u32,
        drag_processed: &'a mut [u32],
        drag_stamp_value: u32,
    ) -> BrushStrokeParamsBuilder<'a> {

        BrushStrokeParamsBuilder {
            start_x: 0,
            start_y: 0,
            end_x: 0,
            end_y: 0,
            canvas,
            color,
            layer,
            visited,
            stamp,
            alpha_overlay: false,
            drag_processed,
            drag_stamp_value,
        }
    }
}

impl<'a> BrushStrokeParamsBuilder<'a> {
    /// Set the line start point.

    pub(crate) fn start(mut self, x: u32, y: u32) -> Self {

        self.start_x = x;

        self.start_y = y;

        self
    }

    /// Set the line end point.

    pub(crate) fn end(mut self, x: u32, y: u32) -> Self {

        self.end_x = x;

        self.end_y = y;

        self
    }

    /// Set alpha-overlay mode (default: `false`).

    pub(crate) fn alpha_overlay(mut self, v: bool) -> Self {

        self.alpha_overlay = v;

        self
    }

    /// Finalise and produce a [`BrushStrokeParams`].

    pub(crate) fn build(self) -> BrushStrokeParams<'a> {

        BrushStrokeParams {
            start_x: self.start_x,
            start_y: self.start_y,
            end_x: self.end_x,
            end_y: self.end_y,
            canvas: self.canvas,
            color: self.color,
            layer: self.layer,
            visited: self.visited,
            stamp: self.stamp,
            alpha_overlay: self.alpha_overlay,
            drag_processed: self.drag_processed,
            drag_stamp_value: self.drag_stamp_value,
        }
    }
}

impl std::fmt::Debug for BrushStrokeParams<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        f.debug_struct("BrushStrokeParams")
            .field("start_x", &self.start_x)
            .field("start_y", &self.start_y)
            .field("end_x", &self.end_x)
            .field("end_y", &self.end_y)
            .field("color", &self.color)
            .field("layer", &self.layer)
            .field("visited.len", &self.visited.len())
            .field("stamp", &self.stamp)
            .field("alpha_overlay", &self.alpha_overlay)
            .field("drag_processed.len", &self.drag_processed.len())
            .field("drag_stamp_value", &self.drag_stamp_value)
            .finish()
    }
}
