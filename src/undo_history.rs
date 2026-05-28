use std::collections::VecDeque;

use crate::canvas::Canvas;
use crate::undo::{ redo_apply, undo_apply, RunSegment, UndoRecord };

const MAX_STROKE_STACK: usize = 1000;

/// Holds accumulated run segments during an active drag gesture.
struct DragAccumulator {
    runs: Vec<RunSegment>,
    layer_index: usize,
    width: u32,
    color_after: eframe::egui::Color32,
    is_alpha_overlay: bool,
}

/// Manages the undo/redo history stack with a visited-stamp buffer for
/// brush-stroke deduplication.
pub struct UndoHistory {
    pub stroke_stack: VecDeque<UndoRecord>,
    pub redo_index: usize,
    pub visited: Vec<u32>,
    pub visited_stamp: u32,
    pub drag_processed: Vec<u32>,
    pub drag_stamp_val: u32,
    drag_accumulator: Option<DragAccumulator>,
}

impl UndoHistory {
    /// Create an empty undo history with a visited-stamp buffer of `pixel_count` zeros.
    ///
    /// `visited_stamp` starts at 1 so that 0 can serve as the "unvisited" sentinel.
    pub fn new(pixel_count: usize) -> Self {
        Self {
            stroke_stack: VecDeque::new(),
            redo_index: 0,
            visited: vec![0u32; pixel_count],
            visited_stamp: 1,
            drag_processed: vec![0u32; pixel_count],
            drag_stamp_val: 1,
            drag_accumulator: None,
        }
    }

    /// Push a new undo record and truncate any redo history.
    ///
    /// Enforces `MAX_STROKE_STACK` by popping the oldest entries.
    pub fn push_undo(&mut self, record: UndoRecord) {
        self.stroke_stack.truncate(self.stroke_stack.len() - self.redo_index);
        self.stroke_stack.push_back(record);
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
    pub fn resize_visited(&mut self, pixel_count: usize) {
        if self.visited.len() < pixel_count {
            self.visited = vec![0u32; pixel_count];
        }
        if self.drag_processed.len() < pixel_count {
            self.drag_processed = vec![0u32; pixel_count];
        }
        self.visited_stamp = 1;
        self.drag_stamp_val = 1;
    }

    /// Advance and return the drag-scoped processed stamp.
    ///
    /// Resets the `drag_processed` buffer when the stamp wraps past `u32::MAX`.
    #[inline]
    pub fn advance_drag_stamp(&mut self) {
        self.drag_stamp_val = self.drag_stamp_val.wrapping_add(1);
        if self.drag_stamp_val == 0 {
            self.drag_processed.fill(0);
            self.drag_stamp_val = 1;
        }
    }

    /// Begin accumulating undo data for a new drag gesture.
    ///
    /// Stores the layer and color metadata; runs are accumulated via
    /// [`extend_drag_accumulator`] and finally pushed as one record via
    /// [`finalize_drag_accumulator`].
    pub fn init_drag_accumulator(&mut self, layer_index: usize, width: u32, color_after: eframe::egui::Color32, is_alpha_overlay: bool) {
        self.drag_accumulator = Some(DragAccumulator {
            runs: Vec::new(),
            layer_index,
            width,
            color_after,
            is_alpha_overlay,
        });
    }

    /// Accumulate a frame's worth of run segments during a drag.
    ///
    /// New runs are **prepended** before previously accumulated runs so that
    /// `undo_apply` processes most-recent runs first. This ensures correct
    /// undo for overlapping non-alpha paint (each step walks back through
    /// intermediate states to the original). For alpha overlay, runs are
    /// disjoint (guaranteed by `drag_processed`), so prepending has no
    /// correctness impact.
    pub fn extend_drag_accumulator(&mut self, runs: Vec<RunSegment>) {
        if let Some(ref mut accumulator) = self.drag_accumulator {
            let mut combined = runs;
            combined.append(&mut accumulator.runs);
            accumulator.runs = combined;
        }
    }

    /// Finish accumulating drag data and push the result as a single undo record.
    ///
    /// No-op if no drag was in progress.
    pub fn finalize_drag_accumulator(&mut self) {
        if let Some(accumulator) = self.drag_accumulator.take() {
            let record = UndoRecord::Run {
                layer_index: accumulator.layer_index,
                color_after: accumulator.color_after,
                runs: accumulator.runs,
                is_alpha_overlay: accumulator.is_alpha_overlay,
            };
            self.push_undo(record);
        }
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
    pub fn undo_step(&mut self, canvas: &mut Canvas, steps_multiplier: usize) {
        let step_count = steps_multiplier.min(self.stroke_stack.len() - self.redo_index);
        for _ in 0..step_count {
            let index = self.stroke_stack.len() - 1 - self.redo_index;
            undo_apply(canvas, &self.stroke_stack[index]);
            self.redo_index += 1;
        }
    }

    /// Re-apply `steps_multiplier` previously undone records from the stack.
    ///
    /// Each record is reapplied in order (oldest undone first),
    /// and `redo_index` decreases accordingly.
    pub fn redo_step(&mut self, canvas: &mut Canvas, steps_multiplier: usize) {
        let step_count = steps_multiplier.min(self.redo_index);
        for _ in 0..step_count {
            let index = self.stroke_stack.len() - self.redo_index;
            self.redo_index -= 1;
            redo_apply(canvas, &self.stroke_stack[index]);
        }
    }
}
