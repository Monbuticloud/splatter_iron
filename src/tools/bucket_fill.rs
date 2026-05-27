use eframe::egui::Color32;

use crate::canvas::{ Canvas, DirtyRect };
use crate::pixel::premultiply;
use crate::undo::{ compress_run, RunSegment, UndoRecord };

/// Fill a contiguous region of matching color starting from the seed point.
///
/// Uses a scanline flood-fill algorithm: for each seed point popped from a
/// stack, it finds the contiguous horizontal span of `target_color`, fills it,
/// and pushes new seed points from the rows above and below where the span
/// intersects new runs of `target_color`.
///
/// Captures before-pixel data for every modified pixel to support undo.
///
/// # Panics
///
/// Panics if `layer >= canvas.pixels.len()`.
#[inline]
pub fn draw_bucket_fill(
    seed_x: u32,
    seed_y: u32,
    canvas: &mut Canvas,
    color: Color32,
    layer: usize,
    alpha_overlay: bool,
) -> UndoRecord {
    let color = premultiply(color);
    let width = canvas.width as usize;
    let height = canvas.height;

    let seed_x = seed_x.min(canvas.width - 1);
    let seed_y = seed_y.min(height - 1);

    let pixels = &mut canvas.pixels[layer].pixels;
    let target = pixels[seed_y as usize * width + seed_x as usize];

    if target == color {
        return UndoRecord::Run {
            layer_index: layer,
            width: canvas.width,
            color_after: color,
            runs: Vec::new(),
            is_alpha_overlay: alpha_overlay,
        };
    }

    let mut runs: Vec<RunSegment> = Vec::new();
    let mut stack: Vec<(u32, u32)> = Vec::new();
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0u32;
    let mut max_y = 0u32;

    stack.push((seed_x, seed_y));

    while let Some((x, y)) = stack.pop() {
        let row_start = (y as usize) * width;

        let mut left = x as i64;
        while left > 0 && pixels[row_start + (left - 1) as usize] == target {
            left -= 1;
        }
        let left = left as u32;

        let mut right = x as i64;
        while right < (width - 1) as i64 && pixels[row_start + (right + 1) as usize] == target {
            right += 1;
        }
        let right = right as u32;

        let span_start = row_start + left as usize;
        let span_end = row_start + right as usize + 1;
        let mut before_pixels = Vec::with_capacity(span_end - span_start);
        before_pixels.extend_from_slice(&pixels[span_start..span_end]);
        let (before, len) = compress_run(before_pixels);
        runs.push(RunSegment {
            start: span_start as u32,
            len,
            before,
        });

        if alpha_overlay {
            for p in pixels[span_start..span_end].iter_mut() {
                *p = crate::pixel::alpha_blend(*p, color);
            }
        } else {
            pixels[span_start..span_end].fill(color);
        }

        if left < min_x { min_x = left; }
        if right > max_x { max_x = right; }
        if y < min_y { min_y = y; }
        if y > max_y { max_y = y; }

        let search_left = if left > 0 { left - 1 } else { 0 };
        let search_right = (right + 1).min(canvas.width - 1);

        // Row above
        if y > 0 {
            let prev_row = (y - 1) as usize * width;
            let mut cx = search_left;
            while cx <= search_right {
                if pixels[prev_row + cx as usize] == target {
                    stack.push((cx, y - 1));
                    while cx <= search_right && pixels[prev_row + cx as usize] == target {
                        cx += 1;
                    }
                } else {
                    cx += 1;
                }
            }
        }

        // Row below
        if y < height - 1 {
            let next_row = (y + 1) as usize * width;
            let mut cx = search_left;
            while cx <= search_right {
                if pixels[next_row + cx as usize] == target {
                    stack.push((cx, y + 1));
                    while cx <= search_right && pixels[next_row + cx as usize] == target {
                        cx += 1;
                    }
                } else {
                    cx += 1;
                }
            }
        }
    }

    if min_x != u32::MAX {
        let rect = DirtyRect::new(min_x, min_y, max_x, max_y);
        canvas.dirty_rect = match canvas.dirty_rect {
            Some(r) => Some(r.union(&rect)),
            None => Some(rect),
        };
    }

    UndoRecord::Run {
        layer_index: layer,
        width: canvas.width,
        color_after: color,
        runs,
        is_alpha_overlay: alpha_overlay,
    }
}
