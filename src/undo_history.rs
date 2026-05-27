use std::collections::VecDeque;

use crate::canvas::Canvas;
use crate::undo::{ undo_apply, redo_apply, UndoRecord };

const MAX_STROKE_STACK: usize = 1000;

pub struct UndoHistory {
    pub stroke_stack: VecDeque<UndoRecord>,
    pub redo_index: usize,
    pub visited: Vec<u32>,
    pub visited_stamp: u32,
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
    /// Automatically wraps around and resets the visited buffer when
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
    /// Resets visited_stamp to 1 after resizing.
    pub fn resize_visited(&mut self, pixel_count: usize) {
        if self.visited.len() < pixel_count {
            self.visited = vec![0u32; pixel_count];
        }
        self.visited_stamp = 1;
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
        let count = steps_multiplier.min(self.stroke_stack.len() - self.redo_index);
        for _ in 0..count {
            let idx = self.stroke_stack.len() - 1 - self.redo_index;
            undo_apply(canvas, &self.stroke_stack[idx]);
            self.redo_index += 1;
        }
    }

    /// Re-apply `steps_multiplier` previously undone records from the stack.
    ///
    /// Each record is reapplied in order (oldest undone first),
    /// and `redo_index` decreases accordingly.
    pub fn redo_step(&mut self, canvas: &mut Canvas, steps_multiplier: usize) {
        let count = steps_multiplier.min(self.redo_index);
        for _ in 0..count {
            let idx = self.stroke_stack.len() - self.redo_index;
            self.redo_index -= 1;
            redo_apply(canvas, &self.stroke_stack[idx]);
        }
    }
}
