use eframe::egui::Color32;

use crate::canvas::Canvas;

pub struct StrokePixel {
    pub index: u32,
    pub color_before: Color32,
    pub color_after: Color32,
}

pub struct RunSegment {
    pub start: u32,
    pub before: Vec<Color32>,
}

pub enum UndoRecord {
    Run {
        layer_index: usize,
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
                let end = (run.start as usize) + run.before.len();
                layer.pixels[run.start as usize..end].copy_from_slice(&run.before);
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
                let end = (run.start as usize) + run.before.len();
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
