//! Shared brush utilities for visited-pixel run capture.
//!
//! Provides \[`apply_visited_runs`\] which is used by `square_brush`,
//! `circle_brush`, and `stamp_brush` to collect before-pixel data,
//! write new colour values, and produce compressed \[`RunSegment`\]s
//! for undo records.

use eframe::egui::Color32;

use crate::canvas::DirtyRect;
use crate::undo::BeforePixels;
use crate::undo::RLE_SHORT_RUN_THRESHOLD;
use crate::undo::RunSegment;

/// Apply color to all visited pixels within a dirty region, capture
/// before-pixels, and return compressed run segments for undo.
///
/// Iterates the dirty rect row by row. For each pixel marked with the
/// current `stamp` value (and not already drag-processed in alpha-overlay
/// mode), captures the old color, writes the new color (blend or replace),
/// and assembles contiguous runs.
///
/// Uniform runs of 8+ pixels use [`BeforePixels::All`] without allocating
/// an intermediate buffer. Non-uniform runs directly extend the caller's
/// flat `before_pixels` buffer.
///
/// # Parameters
///
/// * `pixels` — Mutable layer pixels.
/// * `dirty_rect` — Bounding box of the stamped region.
/// * `width` — Canvas width in pixels.
/// * `visited` — Stamp buffer marking which pixels this stroke touches.
/// * `stamp` — The current stamp value to match against `visited`.
/// * `color` — Colour to apply (premultiplied-alpha).
/// * `alpha_overlay` — If true, alpha-blend instead of overwriting.
/// * `drag_processed` — Per-pixel drag-stamp buffer (for alpha-overlay dedup).
/// * `drag_stamp_value` — The current drag-stamp value.
/// * `before_pixels` — Flat buffer receiving non-uniform before-pixel data.
///
/// # Returns
///
/// A vector of `RunSegment` suitable for embedding in an `UndoRecord::Run`.
///
/// # Panics
///
/// Panics if `pixels` is too small to cover the dirty rect at the given
/// `width`, or if `visited` / `drag_processed` are too small.
#[inline]
pub fn apply_visited_runs(
    pixels: &mut [Color32],
    dirty_rect: &DirtyRect,
    width: usize,
    visited: &[u32],
    stamp: u32,
    color: Color32,
    alpha_overlay: bool,
    drag_processed: &mut [u32],
    drag_stamp_value: u32,
    before_pixels: &mut Vec<Color32>,
) -> Vec<RunSegment> {
    let mut runs: Vec<RunSegment> = Vec::new();

    for y in dirty_rect.min_y..=dirty_rect.max_y {
        let row_start = (y as usize) * width;
        let mut x = dirty_rect.min_x;
        while x <= dirty_rect.max_x {
            let pixel_index = row_start + x as usize;
            if visited[pixel_index] != stamp {
                x += 1;
                continue;
            }
            if alpha_overlay && drag_processed[pixel_index] == drag_stamp_value {
                x += 1;
                continue;
            }
            let run_start = pixel_index as u32;

            // Count contiguous visited pixels in this run.
            let mut run_len: u32 = 0;
            while x <= dirty_rect.max_x {
                let next_pixel_index = row_start + x as usize;
                if visited[next_pixel_index] != stamp {
                    break;
                }
                if alpha_overlay && drag_processed[next_pixel_index] == drag_stamp_value {
                    break;
                }
                run_len += 1;
                x += 1;
            }

            // Check uniformity without allocating an intermediate Vec.
            let run_slice = &pixels[run_start as usize..run_start as usize + run_len as usize];
            let uniform =
                run_len >= RLE_SHORT_RUN_THRESHOLD && run_slice.iter().all(|&p| p == run_slice[0]);

            let (rle_before, length) = if uniform {
                (BeforePixels::All(run_slice[0]), run_len)
            } else {
                let offset = before_pixels.len() as u32;
                before_pixels.extend_from_slice(run_slice);
                (
                    BeforePixels::Many {
                        offset,
                        length: run_len,
                    },
                    run_len,
                )
            };

            // Apply colour.
            let run_slice =
                &mut pixels[run_start as usize..run_start as usize + run_len as usize];
            if alpha_overlay {
                crate::pixel::alpha_blend_span(run_slice, color);
            } else {
                run_slice.fill(color);
            }
            if alpha_overlay {
                let ds = drag_stamp_value;
                for d in
                    &mut drag_processed[run_start as usize..run_start as usize + run_len as usize]
                {
                    *d = ds;
                }
            }

            runs.push(RunSegment {
                start: run_start,
                length,
                before: rle_before,
            });
        }
    }

    runs
}
