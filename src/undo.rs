use eframe::egui::Color32;

use crate::canvas::Canvas;
use crate::pixel::alpha_blend;

/// Compressed storage for a run of before-pixels: either all the same color
/// (`All`) or a full `Vec` of distinct colors (`Many`).
#[derive(Clone)]
pub enum BeforePixels {
    All(Color32),
    Many(Vec<Color32>),
}

/// A contiguous range of pixels in an undo `Run` record.
pub struct RunSegment {
    pub start: u32,
    pub length: u32,
    pub before: BeforePixels,
}

const RLE_SHORT_RUN_THRESHOLD: u32 = 8;

/// Compress a contiguous run of pixel data for efficient undo storage.
///
/// If the run is longer than 8 pixels and all pixels are identical, stores
/// a single color (`BeforePixels::All`) instead of the full vector.
/// Short or non-uniform runs store the full `Vec<Color32>`.
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
        layer_index: usize,
        color_after: Color32,
        runs: Vec<RunSegment>,
        is_alpha_overlay: bool,
    },
}

/// Restore canvas state that was changed by a stroke, using its undo record.
///
/// Restores the saved before-pixels in each run segment.
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
