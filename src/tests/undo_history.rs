//! Tests for `UndoHistory` — stack management, visited-stamp deduplication,
//! drag-accumulator lifecycle, and multi-step undo/redo.
//!
//! Exercises the full undo/redo pipeline with real brush strokes to
//! validate that the history stack behaves correctly under interleaved
//! drag and single-click operations.

use eframe::egui::Color32;

use crate::tests::common::{ red, small_canvas };
use crate::tools::square_brush;
use crate::undo::{ UndoRecord };
use crate::undo_history::UndoHistory;

/// A new undo history should have no undo or redo available.
#[test]
fn new_history_has_no_undo_no_redo() {
    let history = UndoHistory::new(100);
    assert!(!history.can_undo());
    assert!(!history.can_redo());
}

/// Pushing a record should enable undo but not redo.
#[test]
fn push_undo_makes_undo_available() {
    let mut history = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let record = square_brush::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    history.push_undo(record);
    assert!(history.can_undo());
    assert!(!history.can_redo());
}

/// Undoing should restore original pixels and make redo available.
#[test]
fn undo_step_applies_and_tracks_redo() {
    let mut history = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let original = canvas.pixels[0].pixels[0];
    let record = square_brush::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    history.push_undo(record);

    history.undo_step(&mut canvas, 1);
    assert_eq!(canvas.pixels[0].pixels[0], original);
    assert!(history.can_redo());
}

/// Redoing should reapply the undone stroke.
#[test]
fn redo_step_reapplies() {
    let mut history = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let record = square_brush::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    history.push_undo(record);
    history.undo_step(&mut canvas, 1);
    history.redo_step(&mut canvas, 1);
    assert_eq!(canvas.pixels[0].pixels[0], red());
    assert!(!history.can_redo());
}

/// Pushing a new record should clear any redo history.
#[test]
fn push_undo_truncates_redo_stack() {
    let mut history = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let r1 = square_brush::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    history.push_undo(r1);
    history.undo_step(&mut canvas, 1);
    assert!(history.can_redo());

    let r2 = square_brush::draw_square(4, 4, 7, 7, &mut canvas, red(), 0, false);
    history.push_undo(r2);
    assert!(!history.can_redo(), "new stroke should clear redo");
}

/// Undoing and redoing multiple steps at once should work correctly.
#[test]
fn undo_redo_multi_step() {
    let mut history = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let (r1, r2) = {
        let r1 = square_brush::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    let blue = Color32::from_rgba_premultiplied(0, 0, 255, 255);
        let r2 = square_brush::draw_square(3, 3, 6, 6, &mut canvas, blue, 0, false);
        (r1, r2)
    };
    history.push_undo(r1);
    history.push_undo(r2);

    // Undo both at once
    history.undo_step(&mut canvas, 2);
    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::TRANSPARENT,
        "both strokes undone"
    );

    // Redo both at once
    history.redo_step(&mut canvas, 2);
    assert_eq!(canvas.pixels[0].pixels[0], red(), "first stroke back");
}

/// Clearing history should remove all undo/redo.
#[test]
fn clear_resets_history() {
    let mut history = UndoHistory::new(100);
    let mut canvas = small_canvas();
    history.push_undo(square_brush::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false));
    history.push_undo(square_brush::draw_square(4, 4, 7, 7, &mut canvas, red(), 0, false));
    assert!(history.can_undo());
    history.clear();
    assert!(!history.can_undo());
    assert!(!history.can_redo());
}

/// Consecutive stamps should increase by 1.
#[test]
fn next_stamp_increments() {
    let mut history = UndoHistory::new(100);
    let stamp1 = history.next_stamp();
    let stamp2 = history.next_stamp();
    assert_eq!(stamp2, stamp1.wrapping_add(1));
}

/// When the stamp wraps past `u32::MAX`, it should reset to 1.
#[test]
fn stamp_wrapping_overflow_resets() {
    // Force stamp near wrapping point by setting visited_stamp directly
    let mut history = UndoHistory::new(100);
    history.visited_stamp = u32::MAX;
    let stamp = history.next_stamp();
    // wraps to 0, which triggers reset to 1
    assert_eq!(stamp, 1, "wrapping should reset to 1");
    assert_eq!(history.visited.iter().all(|&v| v == 0), true);
}

// --- resize_visited ---

/// `resize_visited` should grow the visited buffer when the canvas gets larger.
#[test]
fn resize_visited_grows_buffer() {
    let mut history = UndoHistory::new(100);
    assert_eq!(history.visited.len(), 100);
    assert_eq!(history.drag_processed.len(), 100);
    history.resize_visited(250);
    assert_eq!(history.visited.len(), 250);
    assert_eq!(history.drag_processed.len(), 250);
    assert_eq!(history.visited_stamp, 1);
    assert_eq!(history.drag_stamp_value, 1);
}

/// `resize_visited` should not shrink the buffer.
#[test]
fn resize_visited_does_not_shrink() {
    let mut history = UndoHistory::new(100);
    history.resize_visited(50);
    assert_eq!(history.visited.len(), 100, "should not shrink");
    assert_eq!(history.drag_processed.len(), 100, "should not shrink");
}

// --- advance_drag_stamp ---

/// `advance_drag_stamp` should increment the stamp value.
#[test]
fn advance_drag_stamp_increments() {
    let mut history = UndoHistory::new(100);
    let before = history.drag_stamp_value;
    history.advance_drag_stamp();
    assert_eq!(
        history.drag_stamp_value,
        before.wrapping_add(1),
        "drag stamp incremented"
    );
}

/// `advance_drag_stamp` should reset the drag_processed buffer when wrapping past u32::MAX.
#[test]
fn advance_drag_stamp_wrapping_resets() {
    let mut history = UndoHistory::new(100);
    history.drag_processed[42] = 1;
    history.drag_stamp_value = u32::MAX;
    history.advance_drag_stamp();
    assert_eq!(history.drag_stamp_value, 1, "wrapping resets to 1");
    assert_eq!(
        history.drag_processed.iter().all(|&v| v == 0),
        true,
        "buffer cleared on wrap"
    );
}

// --- Drag accumulator ---

/// Full drag accumulator lifecycle: init, extend, finalize produces one undo record.
#[test]
fn drag_accumulator_full_lifecycle() {
    let mut history = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let _blue = Color32::from_rgba_premultiplied(0, 0, 255, 255);

    // Simulate a two-frame drag
    history.init_drag_accumulator(0, 10, red(), false);
    let record1 = square_brush::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    let UndoRecord::Run { runs, .. } = record1;
    history.extend_drag_accumulator(runs);
    let record2 = square_brush::draw_square(3, 3, 6, 6, &mut canvas, red(), 0, false);
    let UndoRecord::Run { runs, .. } = record2;
    history.extend_drag_accumulator(runs);
    history.finalize_drag_accumulator();

    assert!(history.can_undo(), "drag should produce one undo record");
    assert!(!history.can_redo(), "no redo yet");

    // Undo should restore original transparent pixels
    history.undo_step(&mut canvas, 1);
    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::TRANSPARENT,
        "undo restored original"
    );
    assert_eq!(
        canvas.pixels[0].pixels[35],
        Color32::TRANSPARENT,
        "undo restored original"
    );
}

// --- Clamping ---

/// `undo_step` with steps_multiplier greater than available undo should clamp.
#[test]
fn undo_step_clamps_to_available() {
    let mut history = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let record = square_brush::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    history.push_undo(record);
    // Request 10 steps, only 1 available — should not panic
    history.undo_step(&mut canvas, 10);
    assert!(!history.can_undo(), "no more undo after clamping");
}

/// `redo_step` with steps_multiplier greater than available redo should clamp.
#[test]
fn redo_step_clamps_to_available() {
    let mut history = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let record = square_brush::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    history.push_undo(record);
    history.undo_step(&mut canvas, 1);
    // Request 10 steps, only 1 available — should not panic
    history.redo_step(&mut canvas, 10);
    assert!(!history.can_redo(), "no more redo after clamping");
}

// --- MAX_STROKE_STACK eviction ---

/// Pushing more than MAX_STROKE_STACK (1000) records should evict the oldest.
#[test]
fn max_stack_eviction_pops_oldest() {
    let mut history = UndoHistory::new(100);
    let mut canvas = small_canvas();
    // Push 1001 records (1000 + 1)
    for _ in 0..1001 {
        let record = square_brush::draw_square(0, 0, 1, 1, &mut canvas, red(), 0, false);
        history.push_undo(record);
    }
    assert_eq!(history.stroke_stack.len(), 1000, "stack capped at 1000");
    assert!(history.can_undo(), "still has undo records");
}

/// `extend_drag_accumulator` without prior `init_drag_accumulator` should be a no-op.
#[test]
fn extend_drag_without_init_noop() {
    let mut history = UndoHistory::new(100);
    // No init was called — extending should not crash or create state
    history.extend_drag_accumulator(Vec::new());
    assert!(!history.can_undo());
}

/// `extend_drag_accumulator` with some runs without init should not crash.
#[test]
fn extend_drag_without_init_with_runs_noop() {
    use crate::undo::{ RunSegment, BeforePixels };
    let mut history = UndoHistory::new(100);
    let runs = vec![RunSegment {
        start: 0,
        length: 5,
        before: BeforePixels::All(Color32::RED),
    }];
    history.extend_drag_accumulator(runs);
    assert!(!history.can_undo());
}

/// `finalize_drag_accumulator` without prior `init_drag_accumulator` should be a no-op.
#[test]
fn finalize_drag_without_init_noop() {
    let mut history = UndoHistory::new(100);
    history.finalize_drag_accumulator();
    assert!(!history.can_undo());
    assert!(!history.can_redo());
}

/// `undo_step` on an empty history should be a no-op.
#[test]
fn undo_step_on_empty_history_noop() {
    let mut history = UndoHistory::new(100);
    let mut canvas = crate::tests::common::small_canvas();
    // Should not panic
    history.undo_step(&mut canvas, 1);
    assert!(!history.can_undo());
}

/// `redo_step` on an empty history should be a no-op.
#[test]
fn redo_step_on_empty_history_noop() {
    let mut history = UndoHistory::new(100);
    let mut canvas = crate::tests::common::small_canvas();
    history.redo_step(&mut canvas, 1);
    assert!(!history.can_redo());
}

/// `push_undo` with zero-length runs (empty record) should still push.
#[test]
fn push_undo_empty_record() {
    use crate::undo::UndoRecord;
    let mut history = UndoHistory::new(100);
    let empty_record = UndoRecord::Run {
        layer_index: 0,
        color_after: Color32::RED,
        runs: Vec::new(),
        is_alpha_overlay: false,
    };
    history.push_undo(empty_record);
    assert!(history.can_undo());
}

/// `resize_visited` with identical size should be a no-op.
#[test]
fn resize_visited_same_size_noop() {
    let mut history = UndoHistory::new(100);
    history.resize_visited(100);
    assert_eq!(history.visited.len(), 100);
    assert_eq!(history.drag_processed.len(), 100);
}
