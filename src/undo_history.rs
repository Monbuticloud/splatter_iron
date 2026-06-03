//! Undo/redo stack with visited-stamp deduplication for brush strokes and
//! a drag accumulator that merges per-frame run segments into a single record.

use std::collections::VecDeque;

use crate::canvas::Canvas;
use crate::canvas::Layer;
use crate::undo::BeforePixels;
use crate::undo::RunSegment;
use crate::undo::UndoRecord;
use crate::undo::redo_apply;
use crate::undo::undo_apply;

const MAX_STROKE_STACK: usize = 1000;
/// Maximum number of frames accumulated in a single drag gesture before
/// the accumulator is force-finalised and a new one starts.
/// Prevents unbounded memory growth during very long drag interactions.
const MAX_DRAG_FRAMES: usize = 5000;

/// Run segments and before-pixels from one drag frame.
#[derive(Debug)]
struct DragFrame {
    runs: Vec<RunSegment>,
    before_pixels: Vec<eframe::egui::Color32>,
}

/// Holds accumulated drag frames during an active drag gesture.
#[derive(Debug)]
struct DragAccumulator {
    frames: Vec<DragFrame>,
    layer_index: usize,
    width: u32,
    color_after: eframe::egui::Color32,
    is_alpha_overlay: bool,
}

/// Manages the undo/redo history stack with a visited-stamp buffer for
/// brush-stroke deduplication.
pub struct UndoHistory {
    /// Stack of undo records, most recent at the back.
    stroke_stack: VecDeque<UndoRecord>,
    /// Index within `stroke_stack` delimiting redo entries (entries >= this
    /// index are redoable).
    redo_index: usize,
    /// Per-pixel stamp counters for deduplication during drag strokes.
    visited: Vec<u32>,
    /// Global stamp counter incremented per stroke.
    visited_stamp: u32,
    /// Per-pixel drag stamps for the current drag gesture.
    drag_processed: Vec<u32>,
    /// Stamp value for the current drag gesture.
    drag_stamp_value: u32,
    drag_accumulator: Option<DragAccumulator>,
}

impl std::fmt::Debug for UndoHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UndoHistory")
            .field("stroke_stack.len", &self.stroke_stack.len())
            .field("redo_index", &self.redo_index)
            .field("visited.len", &self.visited.len())
            .field("visited_stamp", &self.visited_stamp)
            .field("drag_processed.len", &self.drag_processed.len())
            .field("drag_stamp_value", &self.drag_stamp_value)
            .field("drag_accumulator", &self.drag_accumulator)
            .finish()
    }
}

impl UndoHistory {
    /// Create an empty undo history with a visited-stamp buffer of `pixel_count` zeros.
    ///
    /// `visited_stamp` starts at 1 so that 0 can serve as the "unvisited" sentinel.
    ///
    /// # Parameters
    ///
    /// * `pixel_count` — Number of pixels in the canvas (size of visited buffers).
    pub fn new(pixel_count: usize) -> Self {
        Self {
            stroke_stack: VecDeque::new(),
            redo_index: 0,
            visited: vec![0u32; pixel_count],
            visited_stamp: 1,
            drag_processed: vec![0u32; pixel_count],
            drag_stamp_value: 1,
            drag_accumulator: None,
        }
    }

    const COMPRESSION_LEVEL: i32 = -1;

    /// Push a new undo record and truncate any redo history.
    ///
    /// Compresses `before_pixels` via zstd before storing to reduce
    /// memory usage. Enforces `MAX_STROKE_STACK` by popping the oldest
    /// entries.
    ///
    /// # Parameters
    ///
    /// * `record` — The undo record to push onto the history stack.
    ///
    /// # Panics
    ///
    /// Panics if zstd compression fails (should never happen in practice;
    /// would indicate an OOM or corrupted internal state).
    pub fn push_undo(&mut self, record: UndoRecord) {
        self.stroke_stack
            .truncate(self.stroke_stack.len() - self.redo_index);
        self.stroke_stack.push_back(record);
        let last = self.stroke_stack.back_mut().expect("just pushed");
        last.compress_before(Self::COMPRESSION_LEVEL)
            .expect("zstd compression in push_undo");
        while self.stroke_stack.len() > MAX_STROKE_STACK {
            self.stroke_stack.pop_front();
        }
        self.redo_index = 0;
    }

    /// Push a new undo record with an optional full-layer snapshot.
    ///
    /// When `layer` is provided and the stroke covers >50% of the layer
    /// pixels, the per-pixel before-data is replaced with a zstd-compressed
    /// full-layer snapshot for faster undo. Otherwise behaves identically
    /// to [`push_undo`].
    ///
    /// # Parameters
    ///
    /// * `record` — The undo record to push onto the history stack.
    /// * `layer` — The post-stroke layer; the snapshot heuristic is applied
    ///   when available.
    ///
    /// # Panics
    ///
    /// Panics if zstd compression fails (should never happen in practice;
    /// would indicate an OOM or corrupted internal state).
    pub fn push_undo_snapshot(&mut self, record: UndoRecord, layer: &Layer) {
        self.stroke_stack
            .truncate(self.stroke_stack.len() - self.redo_index);
        self.stroke_stack.push_back(record);
        let last = self.stroke_stack.back_mut().expect("just pushed");
        last.maybe_snapshot(layer, Self::COMPRESSION_LEVEL)
            .expect("zstd compression in push_undo_snapshot");
        last.compress_before(Self::COMPRESSION_LEVEL)
            .expect("zstd compression in push_undo_snapshot");
        while self.stroke_stack.len() > MAX_STROKE_STACK {
            self.stroke_stack.pop_front();
        }
        self.redo_index = 0;
    }

    /// Returns the next unique stamp value for brush-line deduplication.
    ///
    /// Automatically wraps to 0 and resets the visited buffer when
    /// the stamp overflows past `u32::MAX`.
    pub fn next_stamp(&mut self) -> u32 {
        self.visited_stamp = self.visited_stamp.wrapping_add(1);
        if self.visited_stamp == 0 {
            self.visited.fill(0);
            self.visited_stamp = 1;
        }
        self.visited_stamp
    }

    /// Grow the visited buffer to accommodate a new canvas size if needed.
    ///
    /// Resets `visited_stamp` to 1 after resizing.
    ///
    /// # Parameters
    ///
    /// * `pixel_count` — Required number of entries in the visited buffers.
    pub fn resize_visited(&mut self, pixel_count: usize) {
        if self.visited.len() < pixel_count {
            self.visited = vec![0u32; pixel_count];
        }
        if self.drag_processed.len() < pixel_count {
            self.drag_processed = vec![0u32; pixel_count];
        }
        self.visited_stamp = 1;
        self.drag_stamp_value = 1;
    }

    /// Return the visited buffer, drag-processed buffer, and current drag stamp
    /// value in one call so callers can pass all three to tool functions without
    /// fighting the borrow checker.
    #[inline]
    pub fn scratch_buffers(&mut self) -> (&mut [u32], &mut [u32], u32) {
        (
            &mut self.visited,
            &mut self.drag_processed,
            self.drag_stamp_value,
        )
    }

    /// Advance and return the drag-scoped processed stamp.
    ///
    /// Resets the `drag_processed` buffer when the stamp wraps past `u32::MAX`.
    #[inline]
    pub fn advance_drag_stamp(&mut self) {
        self.drag_stamp_value = self.drag_stamp_value.wrapping_add(1);
        if self.drag_stamp_value == 0 {
            self.drag_processed.fill(0);
            self.drag_stamp_value = 1;
        }
    }

    /// Begin accumulating undo data for a new drag gesture.
    ///
    /// Stores the layer and color metadata; runs are accumulated via
    /// \[`extend_drag_accumulator`\] and finally pushed as one record via
    /// \[`finalize_drag_accumulator`\].
    ///
    /// # Parameters
    ///
    /// * `layer_index` — Target layer for the drag gesture.
    /// * `width` — Canvas width (for row-stride computations).
    /// * `color_after` — Color applied by the drag stroke.
    /// * `is_alpha_overlay` — Whether the stroke uses alpha overlay.
    pub fn init_drag_accumulator(
        &mut self,
        layer_index: usize,
        width: u32,
        color_after: eframe::egui::Color32,
        is_alpha_overlay: bool,
    ) {
        self.drag_accumulator = Some(DragAccumulator {
            frames: Vec::new(),
            layer_index,
            width,
            color_after,
            is_alpha_overlay,
        });
    }

    /// Accumulate a frame's worth of run segments during a drag.
    ///
    /// Each frame stores both `runs` and its `before_pixels` buffer
    /// separately. On finalize, frames are merged in reverse order
    /// with offset adjustment for correct undo application.
    ///
    /// # Parameters
    ///
    /// * `runs` — Run segments captured during the current drag frame.
    /// * `before_pixels` — Flat before-pixel buffer for this frame.
    pub fn extend_drag_accumulator(
        &mut self,
        runs: Vec<RunSegment>,
        before_pixels: Vec<eframe::egui::Color32>,
    ) {
        let Some(ref mut accumulator) = self.drag_accumulator else {
            return;
        };

        // When the frame limit is reached, flush accumulated frames as one
        // undo record and start a fresh accumulator for remaining frames.
        if accumulator.frames.len() >= MAX_DRAG_FRAMES {
            let layer_index = accumulator.layer_index;
            let _width = accumulator.width;
            let color_after = accumulator.color_after;
            let is_alpha_overlay = accumulator.is_alpha_overlay;
            // Swap out frames, push current frame, finalize into undo stack.
            let mut taken = Vec::with_capacity(MAX_DRAG_FRAMES.min(256));
            std::mem::swap(&mut taken, &mut accumulator.frames);
            taken.push(DragFrame {
                runs,
                before_pixels,
            });
            let record = Self::merge_frames(taken, layer_index, color_after, is_alpha_overlay);
            self.push_undo(record);
            // Reset drag stamp so fresh strokes don't collide with the archived ones.
            self.drag_stamp_value = self.drag_stamp_value.wrapping_add(1);
            return;
        }

        accumulator.frames.push(DragFrame {
            runs,
            before_pixels,
        });
    }

    /// Merge a collection of drag frames into a single undo record.
    ///
    /// Frames are processed in reverse order (most recent first) so that
    /// overlapping paint regions undo correctly. `BeforePixels::Many` offsets
    /// are adjusted to point into the concatenated `before_pixels` buffer.
    fn merge_frames(
        frames: Vec<DragFrame>,
        layer_index: usize,
        color_after: eframe::egui::Color32,
        is_alpha_overlay: bool,
    ) -> UndoRecord {
        let mut all_runs = Vec::new();
        let mut all_before = Vec::new();

        for mut frame in frames.into_iter().rev() {
            let offset_adjust = all_before.len() as u32;
            for run in &mut frame.runs {
                if let BeforePixels::Many { offset, .. } = &mut run.before {
                    *offset += offset_adjust;
                }
            }
            all_runs.append(&mut frame.runs);
            all_before.append(&mut frame.before_pixels);
        }

        UndoRecord::Run {
            layer_index,
            color_after,
            runs: all_runs,
            before_pixels: all_before,
            compressed_before_pixels: None,
            is_alpha_overlay,
            full_layer_before: None,
        }
    }

    /// Finish accumulating drag data and push the result as a single undo record.
    ///
    /// Delegates to [`merge_frames`] to build the record and calls
    /// [`push_undo`] to store it. No-op if no drag was in progress.
    ///
    /// [`push_undo`]: Self::push_undo
    /// [`merge_frames`]: Self::merge_frames
    pub fn finalize_drag_accumulator(&mut self) {
        if let Some(accumulator) = self.drag_accumulator.take() {
            let record = Self::merge_frames(
                accumulator.frames,
                accumulator.layer_index,
                accumulator.color_after,
                accumulator.is_alpha_overlay,
            );
            self.push_undo(record);
        }
    }

    // --- Accessors used by tests ---

    /// Number of entries in the stroke stack.
    #[cfg(test)]
    #[inline]
    pub fn stroke_stack_len(&self) -> usize {
        self.stroke_stack.len()
    }

    /// Length of the visited buffer (number of canvas pixels).
    #[cfg(test)]
    #[inline]
    pub fn visited_len(&self) -> usize {
        self.visited.len()
    }

    /// Length of the drag-processed buffer.
    #[cfg(test)]
    #[inline]
    pub fn drag_processed_len(&self) -> usize {
        self.drag_processed.len()
    }

    /// Current visited stamp value.
    #[cfg(test)]
    #[inline]
    pub fn visited_stamp(&self) -> u32 {
        self.visited_stamp
    }

    /// Set the visited stamp value directly (for testing wrap behaviour).
    #[cfg(test)]
    #[inline]
    pub fn set_visited_stamp(&mut self, val: u32) {
        self.visited_stamp = val;
    }

    /// Current drag stamp value.
    #[cfg(test)]
    #[inline]
    pub fn drag_stamp_value(&self) -> u32 {
        self.drag_stamp_value
    }

    /// Set the drag stamp value directly (for testing wrap behaviour).
    #[cfg(test)]
    #[inline]
    pub fn set_drag_stamp_value(&mut self, val: u32) {
        self.drag_stamp_value = val;
    }

    /// `true` if every entry in the visited buffer is 0.
    #[cfg(test)]
    #[inline]
    pub fn visited_all_zero(&self) -> bool {
        self.visited.iter().all(|&v| v == 0)
    }

    /// Mutable reference to the drag-processed buffer (used by tests).
    #[cfg(test)]
    #[inline]
    pub fn drag_processed_mut(&mut self) -> &mut [u32] {
        &mut self.drag_processed
    }

    /// Clear the entire stroke stack and reset the redo index.
    pub fn clear(&mut self) {
        self.stroke_stack.clear();
        self.redo_index = 0;
    }

    /// Returns `true` if there are strokes that can be undone.
    pub fn can_undo(&self) -> bool {
        self.redo_index < self.stroke_stack.len()
    }

    /// Returns `true` if there are strokes that can be redone.
    pub fn can_redo(&self) -> bool {
        self.redo_index > 0
    }

    /// Apply `steps_multiplier` undo records from the stroke stack.
    ///
    /// Each record is applied in reverse order (most recent first),
    /// and `redo_index` advances accordingly.
    ///
    /// Decompresses `before_pixels` before each application and
    /// re-compresses it afterward to keep memory usage low.
    ///
    /// # Parameters
    ///
    /// * `canvas` — The canvas to restore pixels on.
    /// * `steps_multiplier` — Number of undo steps to apply.
    ///
    /// # Panics
    ///
    /// Panics if any undo record contains corrupt run segments (delegates to
    /// [`undo_apply`] which checks buffer bounds).
    pub fn undo_step(&mut self, canvas: &mut Canvas, steps_multiplier: usize) {
        const MAX_STEPS: usize = 100;
        let step_count = steps_multiplier
            .min(MAX_STEPS)
            .min(self.stroke_stack.len() - self.redo_index);
        for _ in 0..step_count {
            let index = self.stroke_stack.len() - 1 - self.redo_index;
            let record = &mut self.stroke_stack[index];
            record.decompress_before();
            undo_apply(canvas, record);
            record
                .compress_before(Self::COMPRESSION_LEVEL)
                .expect("zstd compression in undo_step");
            self.redo_index += 1;
        }
    }

    /// Re-apply `steps_multiplier` previously undone records from the stack.
    ///
    /// Each record is reapplied in order (oldest undone first),
    /// and `redo_index` decreases accordingly.
    ///
    /// # Parameters
    ///
    /// * `canvas` — The canvas to re-apply strokes on.
    /// * `steps_multiplier` — Number of redo steps to apply.
    ///
    /// # Panics
    ///
    /// Panics if any redo record contains corrupt run segments (delegates to
    /// [`redo_apply`] which checks buffer bounds).
    pub fn redo_step(&mut self, canvas: &mut Canvas, steps_multiplier: usize) {
        const MAX_STEPS: usize = 100;
        let step_count = steps_multiplier.min(MAX_STEPS).min(self.redo_index);
        for _ in 0..step_count {
            let index = self.stroke_stack.len() - self.redo_index;
            self.redo_index -= 1;
            redo_apply(canvas, &self.stroke_stack[index]);
        }
    }
}
