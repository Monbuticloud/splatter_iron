use eframe::egui::Color32;

use crate::canvas::{self, Canvas, Layer};
use crate::undo::{self, BeforePixels, UndoRecord};

/// Short runs below the threshold should return `BeforePixels::Many`.
#[test]
fn compress_run_short_returns_many() {
    let pixels = vec![Color32::RED; 4];
    let (before, len) = undo::compress_run(pixels.clone());
    assert_eq!(len, 4);
    assert!(matches!(before, BeforePixels::Many(_)));
}

/// Long uniform runs should be stored as `BeforePixels::All`.
#[test]
fn compress_run_uniform_long_returns_all() {
    let pixels = vec![Color32::GREEN; 20];
    let (before, len) = undo::compress_run(pixels);
    assert_eq!(len, 20);
    let BeforePixels::All(c) = before else { panic!("expected All") };
    assert_eq!(c, Color32::GREEN);
}

/// Long non-uniform runs should be stored as `BeforePixels::Many`.
#[test]
fn compress_run_mixed_long_returns_many() {
    let pixels: Vec<Color32> = (0..20)
        .map(|i| if i % 2 == 0 { Color32::RED } else { Color32::BLUE })
        .collect();
    let (before, len) = undo::compress_run(pixels);
    assert_eq!(len, 20);
    assert!(matches!(before, BeforePixels::Many(_)));
}

#[test]
fn compress_run_threshold_boundary() {
    // RLE_SHORT_RUN_THRESHOLD = 8
    let uniform = vec![Color32::RED; 8];
    let (_, len) = undo::compress_run(uniform);
    assert_eq!(len, 8, "len 8 should be short → Many");
}

#[test]
fn compress_run_just_above_threshold() {
    let uniform = vec![Color32::RED; 9];
    let (before, len) = undo::compress_run(uniform);
    assert_eq!(len, 9);
    assert!(matches!(before, BeforePixels::All(_)));
}

fn small_white_canvas() -> Canvas {
    Canvas {
        pixels: vec![Layer {
            pixels: vec![Color32::from_rgba_premultiplied(255, 255, 255, 255); 100],
        }],
        height: 10,
        width: 10,
        output_rgba: Vec::new(),
        rendered_layers: None,
        render_next_frame: false,
    }
}

fn red() -> Color32 {
    Color32::from_rgba_premultiplied(255, 0, 0, 255)
}

#[test]
fn undo_apply_restores_before_pixels() {
    let mut canvas = small_white_canvas();
    let original = canvas.pixels[0].pixels[0];
    let record = canvas::draw_square(0, 0, 5, 5, &mut canvas, red(), 0);
    assert_eq!(canvas.pixels[0].pixels[0], red(), "square changed pixel");
    undo::undo_apply(&mut canvas, &record);
    assert_eq!(canvas.pixels[0].pixels[0], original, "undo restored pixel");
}

#[test]
fn redo_apply_reapplies_color() {
    let mut canvas = small_white_canvas();
    let record = canvas::draw_square(0, 0, 5, 5, &mut canvas, red(), 0);
    undo::undo_apply(&mut canvas, &record);
    undo::redo_apply(&mut canvas, &record);
    assert_eq!(canvas.pixels[0].pixels[0], red());
}

#[test]
fn undo_redo_full_roundtrip() {
    let mut canvas = small_white_canvas();
    let original = canvas.pixels[0].pixels[0];
    let record = canvas::draw_square(0, 0, 5, 5, &mut canvas, red(), 0);
    undo::undo_apply(&mut canvas, &record);
    assert_eq!(canvas.pixels[0].pixels[0], original);
    undo::redo_apply(&mut canvas, &record);
    assert_eq!(canvas.pixels[0].pixels[0], red());
    undo::undo_apply(&mut canvas, &record);
    assert_eq!(canvas.pixels[0].pixels[0], original);
}

#[test]
fn undo_record_is_runs_variant() {
    let mut canvas = small_white_canvas();
    let record = canvas::draw_square(2, 2, 7, 7, &mut canvas, red(), 0);
    assert!(matches!(record, UndoRecord::Run { .. }));
}

#[test]
fn empty_square_produces_empty_runs() {
    let mut canvas = small_white_canvas();
    let record = canvas::draw_square(5, 5, 5, 5, &mut canvas, red(), 0);
    if let UndoRecord::Run { runs, .. } = &record {
        assert!(runs.is_empty(), "zero-area rect should produce no runs");
    } else {
        panic!("Expected Run variant");
    }
}

#[test]
fn multiple_undos_stack() {
    let mut canvas = small_white_canvas();
    let r1 = canvas::draw_square(0, 0, 3, 3, &mut canvas, red(), 0);
    let blue = Color32::from_rgba_premultiplied(0, 0, 255, 255);
    let r2 = canvas::draw_square(3, 3, 6, 6, &mut canvas, blue, 0);

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
