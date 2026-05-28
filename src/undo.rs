//! Undo-record types ([`UndoRecord`], [`RunSegment`], [`BeforePixels`])
//! and their application to the canvas ([`undo_apply`], [`redo_apply`]).
//! Run-length compression (`compress_run`) reduces storage for uniform spans.

use eframe::egui::Color32;

use crate::canvas::Canvas;
use crate::pixel::alpha_blend;

/// Compressed storage for a run of before-pixels: either all the same color
/// (`All`) or a full `Vec` of distinct colors (`Many`).
#[derive(Clone)]
pub enum BeforePixels {
    /// Every pixel in the run had the same original color.
    All(Color32),
    /// Pixels had distinct colors (run was compressed from a non-uniform span).
    Many(Vec<Color32>),
}

/// A contiguous range of pixels in an undo `Run` record.
pub struct RunSegment {
    /// Starting pixel index within the layer's flat pixel array.
    pub start: u32,
    /// Number of contiguous pixels in this run.
    pub length: u32,
    /// Original pixel values before the stroke (compressed storage).
    pub before: BeforePixels,
}

const RLE_SHORT_RUN_THRESHOLD: u32 = 8;

/// Compress a contiguous run of pixel data for efficient undo storage.
///
/// If the run is longer than 8 pixels and all pixels are identical, stores
/// a single color (`BeforePixels::All`) instead of the full vector.
/// Short or non-uniform runs store the full `Vec<Color32>`.
///
/// # Parameters
///
/// * `pixels` — Contiguous run of before-pixels to compress.
pub fn compress_run(pixels: Vec<Color32>) -> (BeforePixels, u32) {
    let length = pixels.len() as u32;
    if length < RLE_SHORT_RUN_THRESHOLD {
        return (BeforePixels::Many(pixels), length);
    }
    let first = pixels[0];
    if pixels.iter().all(|&p| p == first) {
        (BeforePixels::All(first), length)
    } else {
        (BeforePixels::Many(pixels), length)
    }
}

/// A record of a single drawing stroke, used for undo/redo.
///
/// `Run` stores runs of contiguous pixels, compressed for efficiency.
pub enum UndoRecord {
    Run {
        /// Index of the layer that was modified.
        layer_index: usize,
        /// Color applied by the stroke (after-state).
        color_after: Color32,
        /// Compressed run-length segments preserving before-pixel data.
        runs: Vec<RunSegment>,
        /// Whether this stroke was drawn as an alpha overlay (vs. opaque).
        is_alpha_overlay: bool,
    },
}

/// Restore canvas state that was changed by a stroke, using its undo record.
///
/// Restores the saved before-pixels in each run segment.
///
/// # Parameters
///
/// * `canvas` — The canvas whose layer pixels will be restored.
/// * `record` — The undo record containing before-pixel data.
///
/// # Panics
///
/// Panics if any run segment extends past the end of the target layer's
/// pixel buffer. This indicates a corrupt or mismatched undo record.
#[inline]
pub fn undo_apply(canvas: &mut Canvas, record: &UndoRecord) {
    let UndoRecord::Run { layer_index, color_after: _, runs, is_alpha_overlay: _ } = record;
    let layer = &mut canvas.pixels[*layer_index];
    for run in runs {
        let end = (run.start as usize) + run.length as usize;
        match &run.before {
            BeforePixels::All(color) => layer.pixels[run.start as usize..end].fill(*color),
            BeforePixels::Many(pixels) => {
                layer.pixels[run.start as usize..end].copy_from_slice(pixels);
            }
        }
    }
}

/// Reapply a previously undone stroke from its undo record.
///
/// Fills the segment range with `color_after`.
///
/// # Parameters
///
/// * `canvas` — The canvas whose layer pixels will be re-stroked.
/// * `record` — The undo record containing after-pixel data.
///
/// # Panics
///
/// Panics if any run segment extends past the end of the target layer's
/// pixel buffer. This indicates a corrupt or mismatched undo record.
#[inline]
pub fn redo_apply(canvas: &mut Canvas, record: &UndoRecord) {
    let UndoRecord::Run { layer_index, color_after, runs, is_alpha_overlay } = record;
    let layer = &mut canvas.pixels[*layer_index];
    if *is_alpha_overlay {
        for run in runs {
            let end = (run.start as usize) + run.length as usize;
            for pixel in layer.pixels[run.start as usize..end].iter_mut() {
                *pixel = alpha_blend(*pixel, *color_after);
            }
        }
    } else {
        for run in runs {
            let end = (run.start as usize) + run.length as usize;
            layer.pixels[run.start as usize..end].fill(*color_after);
        }
    }
}
