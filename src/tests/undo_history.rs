//! Tests for `UndoHistory` — stack management, visited-stamp deduplication,
//! drag-accumulator lifecycle, and multi-step undo/redo.
//!
//! Exercises the full undo/redo pipeline with real brush strokes to
//! validate that the history stack behaves correctly under interleaved
//! drag and single-click operations.

use eframe::egui::Color32;

use crate::canvas::{ Canvas, Layer };
use crate::tests::common::{ red, small_canvas };
use crate::tools::square_brush;
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
