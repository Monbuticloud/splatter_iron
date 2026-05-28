//! Tests for midpoint-circle span fill (`circle_brush::draw_circle`)
//! and interpolated stamp-line (`circle_brush::draw_circle_line`).
//!
//! Verifies correct pixel coverage, alpha overlay, and
//! drag-stamp deduplication across connected strokes.

use eframe::egui::Color32;

use crate::canvas::{ Canvas, Layer };
use crate::tests::common::{ blue, red, small_canvas };
use crate::tools::circle_brush;

// --- draw_circle ---

/// `draw_circle` should fill the region at the specified center.
#[test]
fn draw_circle_fills_radius_one() {
    let mut canvas = small_canvas();
    circle_brush::draw_circle(5, 5, 1, &mut canvas, red(), 0, false);
    // Center pixel should be colored
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 5], red(), "center pixel");
    // Neighbors within radius 1 should be filled
    assert_eq!(canvas.pixels[0].pixels[4 * 10 + 5], red(), "above center");
    assert_eq!(canvas.pixels[0].pixels[6 * 10 + 5], red(), "below center");
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 4], red(), "left of center");
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 6], red(), "right of center");
}

/// Pixels outside the circle should remain transparent.
#[test]
fn draw_circle_leaves_outside_unchanged() {
    let mut canvas = small_canvas();
    circle_brush::draw_circle(5, 5, 2, &mut canvas, red(), 0, false);
    // Far corner should be transparent
    assert_eq!(canvas.pixels[0].pixels[0], Color32::TRANSPARENT);
}

/// Drawing a circle at radius 0 returns an empty undo record (no-op).
#[test]
fn draw_circle_radius_zero() {
    let mut canvas = small_canvas();
    circle_brush::draw_circle(5, 5, 0, &mut canvas, red(), 0, false);
    assert_eq!(
        canvas.pixels[0].pixels[5 * 10 + 5],
        Color32::TRANSPARENT,
        "no pixel colored at radius 0"
    );
}

/// A circle centered at origin with radius should not panic.
#[test]
fn draw_circle_at_origin() {
    let mut canvas = small_canvas();
    circle_brush::draw_circle(0, 0, 3, &mut canvas, red(), 0, false);
    assert_eq!(canvas.pixels[0].pixels[0], red(), "origin pixel");
}

/// Drawing on different layers should independently modify each layer.
#[test]
fn draw_circle_multi_layer() {
    let mut canvas = Canvas {
        pixels: vec![
            Layer { pixels: vec![Color32::TRANSPARENT; 100] },
            Layer { pixels: vec![Color32::TRANSPARENT; 100] },
        ],
        height: 10,
        width: 10,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: None,
        render_next_frame: false,
    };
    circle_brush::draw_circle(2, 2, 1, &mut canvas, red(), 0, false);
    circle_brush::draw_circle(7, 7, 1, &mut canvas, blue(), 1, false);
    assert_eq!(canvas.pixels[0].pixels[2 * 10 + 2], red(), "layer 0 has red");
    assert_eq!(
        canvas.pixels[1].pixels[7 * 10 + 7],
        blue(),
        "layer 1 has blue"
    );
}

// --- draw_circle_line ---

/// A horizontal circle-brush line should color pixels at both endrag_processedoints.
#[test]
fn draw_circle_line_horizontal() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    circle_brush::draw_circle_line(1, 5, 8, 5, 0, &mut canvas, red(), 0, &mut visited, 1, false, &mut drag_processed, 0);
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 1], red(), "start");
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 8], red(), "end");
}

/// A vertical circle-brush line should color pixels at both endrag_processedoints.
#[test]
fn draw_circle_line_vertical() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    circle_brush::draw_circle_line(5, 1, 5, 8, 0, &mut canvas, red(), 0, &mut visited, 1, false, &mut drag_processed, 0);
    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 5], red(), "start");
    assert_eq!(canvas.pixels[0].pixels[8 * 10 + 5], red(), "end");
}

/// A diagonal circle-brush line should color pixels at both endrag_processedoints.
#[test]
fn draw_circle_line_diagonal() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    circle_brush::draw_circle_line(1, 1, 8, 8, 0, &mut canvas, red(), 0, &mut visited, 1, false, &mut drag_processed, 0);
    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 1], red(), "start");
    assert_eq!(canvas.pixels[0].pixels[8 * 10 + 8], red(), "end");
}

/// Different stamp values should produce independent brush lines.
#[test]
fn draw_circle_line_different_stamps_dont_interfere() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    circle_brush::draw_circle_line(1, 1, 3, 1, 0, &mut canvas, red(), 0, &mut visited, 1, false, &mut drag_processed, 0);
    circle_brush::draw_circle_line(
        6,
        6,
        8,
        6,
        0,
        &mut canvas,
        blue(),
        0,
        &mut visited,
        2,
        false,
        &mut drag_processed,
        0,
    );
    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 1], red(), "stamp 1");
    assert_eq!(canvas.pixels[0].pixels[6 * 10 + 6], blue(), "stamp 2");
}

/// Circle brush radius should affect the area covered around the cursor.
#[test]
fn draw_circle_line_brush_radius() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    circle_brush::draw_circle_line(5, 5, 5, 5, 3, &mut canvas, red(), 0, &mut visited, 1, false, &mut drag_processed, 0);
    // Center and nearby should be colored
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 5], red(), "center");
    assert_eq!(canvas.pixels[0].pixels[4 * 10 + 5], red(), "above center");
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 4], red(), "left of center");
}

/// Drawing at the canvas edge with a large circle brush should not panic.
#[test]
fn draw_circle_line_clamps_to_canvas() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    circle_brush::draw_circle_line(0, 0, 0, 0, 5, &mut canvas, red(), 0, &mut visited, 1, false, &mut drag_processed, 0);
    assert_eq!(canvas.pixels[0].pixels[0], red(), "corner colored");
}

// --------------------------------------------------
//  Regression: semi-transparent premultiplied color
// --------------------------------------------------

/// Semi-transparent premultiplied circle must be stored as-is (not double-premultiplied).
#[test]
fn draw_circle_preserves_premultiplied_semi_transparent() {
    let mut canvas = small_canvas();
    let semi = Color32::from_rgba_premultiplied(128, 64, 32, 128);
    circle_brush::draw_circle(5, 5, 1, &mut canvas, semi, 0, false);
    assert_eq!(
        canvas.pixels[0].pixels[5 * 10 + 5],
        semi,
        "center pixel should store exact premultiplied color"
    );
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 5].r(), 128, "r must not be darkend");
}

/// Semi-transparent premultiplied circle line must be stored as-is.
#[test]
fn draw_circle_line_preserves_premultiplied_semi_transparent() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    let semi = Color32::from_rgba_premultiplied(128, 64, 32, 128);
    circle_brush::draw_circle_line(
        2, 5, 7, 5, 0, &mut canvas, semi, 0, &mut visited, 1, false, &mut drag_processed, 0,
    );
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 2], semi);
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 2].r(), 128);
}
