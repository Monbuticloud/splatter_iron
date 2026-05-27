use eframe::egui::Color32;

use crate::canvas::{self, Canvas, Layer};
use crate::undo_history::UndoHistory;

/// Build a 10×10 transparent canvas for use in tests.
fn small_canvas() -> Canvas {
    Canvas {
        pixels: vec![Layer {
            pixels: vec![Color32::TRANSPARENT; 100],
        }],
        height: 10,
        width: 10,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: None,
        render_next_frame: false,
    }
}

/// Shortcut for a fully opaque red color in premultiplied format.
fn red() -> Color32 {
    Color32::from_rgba_premultiplied(255, 0, 0, 255)
}

/// A new undo history should have no undo or redo available.
#[test]
fn new_history_has_no_undo_no_redo() {
    let hist = UndoHistory::new(100);
    assert!(!hist.can_undo());
    assert!(!hist.can_redo());
}

/// Pushing a record should enable undo but not redo.
#[test]
fn push_undo_makes_undo_available() {
    let mut hist = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let record = canvas::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    hist.push_undo(record);
    assert!(hist.can_undo());
    assert!(!hist.can_redo());
}

/// Undoing should restore original pixels and make redo available.
#[test]
fn undo_step_applies_and_tracks_redo() {
    let mut hist = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let original = canvas.pixels[0].pixels[0];
    let record = canvas::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    hist.push_undo(record);

    hist.undo_step(&mut canvas, 1);
    assert_eq!(canvas.pixels[0].pixels[0], original);
    assert!(hist.can_redo());
}

/// Redoing should reapply the undone stroke.
#[test]
fn redo_step_reapplies() {
    let mut hist = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let record = canvas::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    hist.push_undo(record);
    hist.undo_step(&mut canvas, 1);
    hist.redo_step(&mut canvas, 1);
    assert_eq!(canvas.pixels[0].pixels[0], red());
    assert!(!hist.can_redo());
}

/// Pushing a new record should clear any redo history.
#[test]
fn push_undo_truncates_redo_stack() {
    let mut hist = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let r1 = canvas::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    hist.push_undo(r1);
    hist.undo_step(&mut canvas, 1);
    assert!(hist.can_redo());

    let r2 = canvas::draw_square(4, 4, 7, 7, &mut canvas, red(), 0, false);
    hist.push_undo(r2);
    assert!(!hist.can_redo(), "new stroke should clear redo");
}

/// Undoing and redoing multiple steps at once should work correctly.
#[test]
fn undo_redo_multi_step() {
    let mut hist = UndoHistory::new(100);
    let mut canvas = small_canvas();
    let (r1, r2) = {
        let r1 = canvas::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
        let blue = Color32::from_rgba_premultiplied(0, 0, 255, 255);
        let r2 = canvas::draw_square(3, 3, 6, 6, &mut canvas, blue, 0, false);
        (r1, r2)
    };
    hist.push_undo(r1);
    hist.push_undo(r2);

    // Undo both at once
    hist.undo_step(&mut canvas, 2);
    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::TRANSPARENT,
        "both strokes undone"
    );

    // Redo both at once
    hist.redo_step(&mut canvas, 2);
    assert_eq!(canvas.pixels[0].pixels[0], red(), "first stroke back");
}

/// Clearing history should remove all undo/redo.
#[test]
fn clear_resets_history() {
    let mut hist = UndoHistory::new(100);
    let mut canvas = small_canvas();
    hist.push_undo(canvas::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false));
    hist.push_undo(canvas::draw_square(4, 4, 7, 7, &mut canvas, red(), 0, false));
    assert!(hist.can_undo());
    hist.clear();
    assert!(!hist.can_undo());
    assert!(!hist.can_redo());
}

/// Consecutive stamps should increase by 1.
#[test]
fn next_stamp_increments() {
    let mut hist = UndoHistory::new(100);
    let s1 = hist.next_stamp();
    let s2 = hist.next_stamp();
    assert_eq!(s2, s1.wrapping_add(1));
}

/// When the stamp wraps past `u32::MAX`, it should reset to 1.
#[test]
fn stamp_wrapping_overflow_resets() {
    // Force stamp near wrapping point by setting visited_stamp directly
    let mut hist = UndoHistory::new(100);
    hist.visited_stamp = u32::MAX;
    let s = hist.next_stamp();
    // wraps to 0, which triggers reset to 1
    assert_eq!(s, 1, "wrapping should reset to 1");
    assert_eq!(hist.visited.iter().all(|&v| v == 0), true);
}
