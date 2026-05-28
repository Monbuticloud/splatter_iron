//! Tests for per-pixel undo/redo record application (`undo_apply`, `redo_apply`).
//!
//! Verifies that stroke records correctly restore original pixels on undo
//! and re-apply the stroke again on redo, including alpha-overlay records.

use eframe::egui::Color32;

use crate::canvas::{ Canvas, Layer };
use crate::tests::common::red;
use crate::tools::square_brush;
use crate::undo::{ self, BeforePixels, UndoRecord };

/// Short runs below the threshold should return `BeforePixels::Many`.
#[test]
fn compress_run_short_returns_many() {
    let pixels = vec![Color32::RED; 4];
    let (before, length) = undo::compress_run(pixels.clone());
    assert_eq!(length, 4);
    assert!(matches!(before, BeforePixels::Many(_)));
}

/// Long uniform runs should be stored as `BeforePixels::All`.
#[test]
fn compress_run_uniform_long_returns_all() {
    let pixels = vec![Color32::GREEN; 20];
    let (before, length) = undo::compress_run(pixels);
    assert_eq!(length, 20);
    let BeforePixels::All(c) = before else { panic!("expected All") };
    assert_eq!(c, Color32::GREEN);
}

/// Long non-uniform runs should be stored as `BeforePixels::Many`.
#[test]
fn compress_run_mixed_long_returns_many() {
    let pixels: Vec<Color32> = (0..20)
        .map(|i| if i % 2 == 0 { Color32::RED } else { Color32::BLUE })
        .collect();
    let (before, length) = undo::compress_run(pixels);
    assert_eq!(length, 20);
    assert!(matches!(before, BeforePixels::Many(_)));
}

/// A run at exactly the threshold length should still be `Many`.
#[test]
fn compress_run_threshold_boundary() {
    // RLE_SHORT_RUN_THRESHOLD = 8
    let uniform = vec![Color32::RED; 8];
    let (_, length) = undo::compress_run(uniform);
    assert_eq!(length, 8, "len 8 should be short → Many");
}

/// A uniform run just above the threshold should be stored as `All`.
#[test]
fn compress_run_just_above_threshold() {
    let uniform = vec![Color32::RED; 9];
    let (before, length) = undo::compress_run(uniform);
    assert_eq!(length, 9);
    assert!(matches!(before, BeforePixels::All(_)));
}

/// Build a 10×10 fully opaque white canvas for use in tests.
fn small_white_canvas() -> Canvas {
    Canvas {
        pixels: vec![Layer {
            pixels: vec![Color32::from_rgba_premultiplied(255, 255, 255, 255); 100],
        }],
        height: 10,
        width: 10,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: None,
        render_next_frame: false,
    }
}

/// `undo_apply` should restore the pixels that were present before the stroke.
#[test]
fn undo_apply_restores_before_pixels() {
    let mut canvas = small_white_canvas();
    let original = canvas.pixels[0].pixels[0];
    let record = square_brush::draw_square(0, 0, 5, 5, &mut canvas, red(), 0, false);
    assert_eq!(canvas.pixels[0].pixels[0], red(), "square changed pixel");
    undo::undo_apply(&mut canvas, &record);
    assert_eq!(canvas.pixels[0].pixels[0], original, "undo restored pixel");
}

/// `redo_apply` should reapply the stroke color.
#[test]
fn redo_apply_reapplies_color() {
    let mut canvas = small_white_canvas();
    let record = square_brush::draw_square(0, 0, 5, 5, &mut canvas, red(), 0, false);
    undo::undo_apply(&mut canvas, &record);
    undo::redo_apply(&mut canvas, &record);
    assert_eq!(canvas.pixels[0].pixels[0], red());
}

/// A full undo → redo → undo roundtrip should restore the original state.
#[test]
fn undo_redo_full_roundtrip() {
    let mut canvas = small_white_canvas();
    let original = canvas.pixels[0].pixels[0];
    let record = square_brush::draw_square(0, 0, 5, 5, &mut canvas, red(), 0, false);
    undo::undo_apply(&mut canvas, &record);
    assert_eq!(canvas.pixels[0].pixels[0], original);
    undo::redo_apply(&mut canvas, &record);
    assert_eq!(canvas.pixels[0].pixels[0], red());
    undo::undo_apply(&mut canvas, &record);
    assert_eq!(canvas.pixels[0].pixels[0], original);
}

/// `draw_square` should produce an `UndoRecord::Run` variant.
#[test]
fn undo_record_is_runs_variant() {
    let mut canvas = small_white_canvas();
    let record = square_brush::draw_square(2, 2, 7, 7, &mut canvas, red(), 0, false);
    assert!(matches!(record, UndoRecord::Run { .. }));
}

/// A zero-area square should produce an undo record with no runs.
#[test]
fn empty_square_produces_empty_runs() {
    let mut canvas = small_white_canvas();
    let record = square_brush::draw_square(5, 5, 5, 5, &mut canvas, red(), 0, false);
    let UndoRecord::Run { runs, .. } = &record;
    assert!(runs.is_empty(), "zero-area rect should produce no runs");
}

/// An empty pixel vec should produce length 0 and `Many` variant.
#[test]
fn compress_run_empty_returns_many() {
    let (before, length) = undo::compress_run(Vec::new());
    assert_eq!(length, 0);
    assert!(matches!(before, BeforePixels::Many(_)));
}

/// `redo_apply` with alpha overlay should blend instead of overwriting.
#[test]
fn redo_apply_alpha_overlay_blends() {
    let mut canvas = small_white_canvas();
    let semi = Color32::from_rgba_premultiplied(128, 0, 0, 128);
    let record = square_brush::draw_square(0, 0, 5, 5, &mut canvas, semi, 0, true);
    // After draw: pixels are alpha-blended semi-transparent red over white
    let after_draw = canvas.pixels[0].pixels[0];
    undo::undo_apply(&mut canvas, &record);
    // After undo: original white should be restored
    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::from_rgba_premultiplied(255, 255, 255, 255)
    );
    undo::redo_apply(&mut canvas, &record);
    // After redo: should match the blended result
    assert_eq!(canvas.pixels[0].pixels[0], after_draw, "redo blend matches original blend");
}

/// `undo_apply` with a uniform run stored as `BeforePixels::All` should restore correctly.
#[test]
fn undo_apply_before_pixels_all_restores() {
    let mut canvas = small_white_canvas();
    // Draw a large enough square (all 100 pixels) to trigger RLE → `All` compression
    let record = square_brush::draw_square(0, 0, 10, 10, &mut canvas, red(), 0, false);
    let UndoRecord::Run { runs, .. } = &record;
    for run in runs {
        assert!(
            matches!(run.before, BeforePixels::All(_)),
            "uniform run should compress to All"
        );
    }
    undo::undo_apply(&mut canvas, &record);
    // All pixels should be restored to original white
    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::from_rgba_premultiplied(255, 255, 255, 255)
    );
    assert_eq!(
        canvas.pixels[0].pixels[99],
        Color32::from_rgba_premultiplied(255, 255, 255, 255)
    );
}

/// Multiple undo/redo operations should compose correctly.
#[test]
fn multiple_undos_stack() {
    let mut canvas = small_white_canvas();
    let r1 = square_brush::draw_square(0, 0, 3, 3, &mut canvas, red(), 0, false);
    let blue = Color32::from_rgba_premultiplied(0, 0, 255, 255);
    let r2 = square_brush::draw_square(3, 3, 6, 6, &mut canvas, blue, 0, false);

    // Undo second stroke
    undo::undo_apply(&mut canvas, &r2);
    assert_eq!(canvas.pixels[0].pixels[0], red(), "first square still present");
    assert_eq!(canvas.pixels[0].pixels[33], Color32::from_rgba_premultiplied(255, 255, 255, 255),
        "second square area restored after undo");

    // Undo first stroke
    undo::undo_apply(&mut canvas, &r1);
    assert_eq!(canvas.pixels[0].pixels[0], Color32::from_rgba_premultiplied(255, 255, 255, 255),
        "all pixels restored after both undos");

    // Redo both in order
    undo::redo_apply(&mut canvas, &r1);
    undo::redo_apply(&mut canvas, &r2);
    assert_eq!(canvas.pixels[0].pixels[0], red(), "first square reapplied");
    assert_eq!(canvas.pixels[0].pixels[33], blue, "second square reapplied");
}
