use eframe::egui::Color32;

use crate::canvas::Canvas;
use crate::pixel::alpha_blend;

/// A single pixel change with its before and after colors (for the `Pixel` undo variant).
pub struct StrokePixel {
    pub index: u32,
    pub color_before: Color32,
    pub color_after: Color32,
}

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
    pub len: u32,
    pub before: BeforePixels,
}

const RLE_SHORT_RUN_THRESHOLD: u32 = 8;

/// Compress a contiguous run of pixel data for efficient undo storage.
///
/// If the run is longer than 8 pixels and all pixels are identical, stores
/// a single color (`BeforePixels::All`) instead of the full vector.
/// Short or non-uniform runs store the full `Vec<Color32>`.
pub fn compress_run(pixels: Vec<Color32>) -> (BeforePixels, u32) {
    let len = pixels.len() as u32;
    if len < RLE_SHORT_RUN_THRESHOLD {
        return (BeforePixels::Many(pixels), len);
    }
    let first = pixels[0];
    if pixels.iter().all(|&p| p == first) {
        (BeforePixels::All(first), len)
    } else {
        (BeforePixels::Many(pixels), len)
    }
}

/// A record of a single drawing stroke, used for undo/redo.
///
/// `Run` stores runs of contiguous pixels, compressed for efficiency.
/// `Pixel` stores individual pixel changes (legacy variant).
#[allow(dead_code)]
pub enum UndoRecord {
    Run {
        layer_index: usize,
        #[allow(dead_code)]
        width: u32,
        color_after: Color32,
        runs: Vec<RunSegment>,
        is_alpha_overlay: bool,
    },
    Pixel {
        layer_index: usize,
        width: u32,
        pixels: Vec<StrokePixel>,
    },
}

/// Restore canvas state that was changed by a stroke, using its undo record.
///
/// For the `Run` variant, restores the saved before-pixels in each run segment.
/// For the `Pixel` variant, writes back each pixel's `color_before`.
#[inline]
pub fn undo_apply(canvas: &mut Canvas, record: &UndoRecord) {
    match record {
        UndoRecord::Run { layer_index, width: _, color_after: _, runs, is_alpha_overlay: _ } => {
            let layer = &mut canvas.pixels[*layer_index];
            for run in runs {
                let end = (run.start as usize) + run.len as usize;
                match &run.before {
                    BeforePixels::All(c) => layer.pixels[run.start as usize..end].fill(*c),
                    BeforePixels::Many(v) => {
                        layer.pixels[run.start as usize..end].copy_from_slice(v);
                    }
                }
            }
        }
        UndoRecord::Pixel { layer_index, width: _, pixels } => {
            let layer = &mut canvas.pixels[*layer_index];
            for pixel in pixels {
                layer.pixels[pixel.index as usize] = pixel.color_before;
            }
        }
    }
}

/// Reapply a previously undone stroke from its undo record.
///
/// For the `Run` variant, fills the segment range with `color_after`.
/// For the `Pixel` variant, writes back each pixel's `color_after`.
#[inline]
pub fn redo_apply(canvas: &mut Canvas, record: &UndoRecord) {
    match record {
        UndoRecord::Run { layer_index, width: _, color_after, runs, is_alpha_overlay } => {
            let layer = &mut canvas.pixels[*layer_index];
            if *is_alpha_overlay {
                for run in runs {
                    let end = (run.start as usize) + run.len as usize;
                    for pixel in layer.pixels[run.start as usize..end].iter_mut() {
                        *pixel = alpha_blend(*pixel, *color_after);
                    }
                }
            } else {
                for run in runs {
                    let end = (run.start as usize) + run.len as usize;
                    layer.pixels[run.start as usize..end].fill(*color_after);
                }
            }
        }
        UndoRecord::Pixel { layer_index, width: _, pixels } => {
            let layer = &mut canvas.pixels[*layer_index];
            for pixel in pixels {
                layer.pixels[pixel.index as usize] = pixel.color_after;
            }
        }
    }
}
