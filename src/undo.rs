use eframe::egui::Color32;

use crate::canvas::Canvas;

pub struct StrokePixel {
    pub index: u32,
    pub color_before: Color32,
    pub color_after: Color32,
}

#[derive(Clone)]
pub enum BeforePixels {
    All(Color32),
    Many(Vec<Color32>),
}

pub struct RunSegment {
    pub start: u32,
    pub len: u32,
    pub before: BeforePixels,
}

const RLE_SHORT_RUN_THRESHOLD: u32 = 8;

/// Compress a captured run of pixels: if uniform and long enough,
/// store as `All(color)` instead of a full `Vec`.
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

#[allow(dead_code)]
pub enum UndoRecord {
    Run {
        layer_index: usize,
        #[allow(dead_code)]
        width: u32,
        color_after: Color32,
        runs: Vec<RunSegment>,
    },
    Pixel {
        layer_index: usize,
        width: u32,
        pixels: Vec<StrokePixel>,
    },
}

#[inline]
pub fn undo_apply(canvas: &mut Canvas, record: &UndoRecord) {
    match record {
        UndoRecord::Run { layer_index, width: _, color_after: _, runs } => {
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

#[inline]
pub fn redo_apply(canvas: &mut Canvas, record: &UndoRecord) {
    match record {
        UndoRecord::Run { layer_index, width: _, color_after, runs } => {
            let layer = &mut canvas.pixels[*layer_index];
            for run in runs {
                let end = (run.start as usize) + run.len as usize;
                layer.pixels[run.start as usize..end].fill(*color_after);
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
