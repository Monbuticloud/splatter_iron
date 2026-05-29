//! Tests for midpoint-circle span fill (`circle_brush::draw_circle`)
//! and interpolated stamp-line (`circle_brush::draw_circle_line`).
//!
//! Verifies correct pixel coverage, alpha overlay, and
//! drag-stamp deduplication across connected strokes.

use eframe::egui::Color32;

use crate::brush_params::BrushStrokeParams;
use crate::canvas::Canvas;
use crate::canvas::DirtyRectList;
use crate::canvas::Layer;
use crate::tests::common::blue;
use crate::tests::common::red;
use crate::tests::common::small_canvas;
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
    assert_eq!(
        canvas.pixels[0].pixels[5 * 10 + 6],
        red(),
        "right of center"
    );
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
            Layer {
                pixels: vec![Color32::TRANSPARENT; 100],
                ..Default::default()
            },
            Layer {
                pixels: vec![Color32::TRANSPARENT; 100],
                ..Default::default()
            },
        ],
        height: 10,
        width: 10,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
    };
    circle_brush::draw_circle(2, 2, 1, &mut canvas, red(), 0, false);
    circle_brush::draw_circle(7, 7, 1, &mut canvas, blue(), 1, false);
    assert_eq!(
        canvas.pixels[0].pixels[2 * 10 + 2],
        red(),
        "layer 0 has red"
    );
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
    circle_brush::draw_circle_line(
        BrushStrokeParams {
            start_x: 1,
            start_y: 5,
            end_x: 8,
            end_y: 5,
            canvas: &mut canvas,
            color: red(),
            layer: 0,
            visited: &mut visited,
            stamp: 1,
            alpha_overlay: false,
            drag_processed: &mut drag_processed,
            drag_stamp_value: 0,
        },
        0,
    );
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 1], red(), "start");
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 8], red(), "end");
}

/// A vertical circle-brush line should color pixels at both endrag_processedoints.
#[test]
fn draw_circle_line_vertical() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    circle_brush::draw_circle_line(
        BrushStrokeParams {
            start_x: 5,
            start_y: 1,
            end_x: 5,
            end_y: 8,
            canvas: &mut canvas,
            color: red(),
            layer: 0,
            visited: &mut visited,
            stamp: 1,
            alpha_overlay: false,
            drag_processed: &mut drag_processed,
            drag_stamp_value: 0,
        },
        0,
    );
    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 5], red(), "start");
    assert_eq!(canvas.pixels[0].pixels[8 * 10 + 5], red(), "end");
}

/// A diagonal circle-brush line should color pixels at both endrag_processedoints.
#[test]
fn draw_circle_line_diagonal() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    circle_brush::draw_circle_line(
        BrushStrokeParams {
            start_x: 1,
            start_y: 1,
            end_x: 8,
            end_y: 8,
            canvas: &mut canvas,
            color: red(),
            layer: 0,
            visited: &mut visited,
            stamp: 1,
            alpha_overlay: false,
            drag_processed: &mut drag_processed,
            drag_stamp_value: 0,
        },
        0,
    );
    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 1], red(), "start");
    assert_eq!(canvas.pixels[0].pixels[8 * 10 + 8], red(), "end");
}

/// Different stamp values should produce independent brush lines.
#[test]
fn draw_circle_line_different_stamps_dont_interfere() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    circle_brush::draw_circle_line(
        BrushStrokeParams {
            start_x: 1,
            start_y: 1,
            end_x: 3,
            end_y: 1,
            canvas: &mut canvas,
            color: red(),
            layer: 0,
            visited: &mut visited,
            stamp: 1,
            alpha_overlay: false,
            drag_processed: &mut drag_processed,
            drag_stamp_value: 0,
        },
        0,
    );
    circle_brush::draw_circle_line(
        BrushStrokeParams {
            start_x: 6,
            start_y: 6,
            end_x: 8,
            end_y: 6,
            canvas: &mut canvas,
            color: blue(),
            layer: 0,
            visited: &mut visited,
            stamp: 2,
            alpha_overlay: false,
            drag_processed: &mut drag_processed,
            drag_stamp_value: 0,
        },
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
    circle_brush::draw_circle_line(
        BrushStrokeParams {
            start_x: 5,
            start_y: 5,
            end_x: 5,
            end_y: 5,
            canvas: &mut canvas,
            color: red(),
            layer: 0,
            visited: &mut visited,
            stamp: 1,
            alpha_overlay: false,
            drag_processed: &mut drag_processed,
            drag_stamp_value: 0,
        },
        3,
    );
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
    circle_brush::draw_circle_line(
        BrushStrokeParams {
            start_x: 0,
            start_y: 0,
            end_x: 0,
            end_y: 0,
            canvas: &mut canvas,
            color: red(),
            layer: 0,
            visited: &mut visited,
            stamp: 1,
            alpha_overlay: false,
            drag_processed: &mut drag_processed,
            drag_stamp_value: 0,
        },
        5,
    );
    assert_eq!(canvas.pixels[0].pixels[0], red(), "corner colored");
}

// --- Alpha overlay ---

/// Alpha overlay mode for `draw_circle` should blend instead of overwriting.
#[test]
fn draw_circle_alpha_overlay_blends() {
    let mut canvas = small_canvas();
    // Pre-fill pixel at center with opaque white
    canvas.pixels[0].pixels[5 * 10 + 5] = Color32::from_rgba_premultiplied(255, 255, 255, 255);
    let semi_red = Color32::from_rgba_premultiplied(128, 0, 0, 128);
    circle_brush::draw_circle(5, 5, 1, &mut canvas, semi_red, 0, true);
    let blended = canvas.pixels[0].pixels[5 * 10 + 5];
    // Blended result should differ from both pure white and pure semi_red
    assert_ne!(
        blended,
        Color32::from_rgba_premultiplied(255, 255, 255, 255),
        "alpha overlay changed pixel"
    );
    assert_ne!(blended, semi_red, "alpha overlay blended, not replaced");
}

/// Alpha overlay mode for `draw_circle_line` should blend instead of overwriting.
#[test]
fn draw_circle_line_alpha_overlay_blends() {
    let mut canvas = small_canvas();
    canvas.pixels[0].pixels[5 * 10 + 1] = Color32::from_rgba_premultiplied(255, 255, 255, 255);
    let mut visited = vec![0u32; 100];
    let mut drag_processed = vec![0u32; 100];
    let semi_red = Color32::from_rgba_premultiplied(128, 0, 0, 128);
    circle_brush::draw_circle_line(
        BrushStrokeParams {
            start_x: 1,
            start_y: 5,
            end_x: 1,
            end_y: 5,
            canvas: &mut canvas,
            color: semi_red,
            layer: 0,
            visited: &mut visited,
            stamp: 1,
            alpha_overlay: true,
            drag_processed: &mut drag_processed,
            drag_stamp_value: 1,
        },
        0,
    );
    let blended = canvas.pixels[0].pixels[5 * 10 + 1];
    assert_ne!(
        blended,
        Color32::from_rgba_premultiplied(255, 255, 255, 255),
        "alpha overlay changed pixel"
    );
    assert_ne!(blended, semi_red, "alpha overlay blended");
}

/// A circle center clamped to the canvas edge should still produce runs
/// (drawn at the nearest valid pixel).
#[test]
fn draw_circle_clamped_to_edge_produces_runs() {
    let mut canvas = small_canvas();
    // Center at (100, 100) — well outside the 10x10 canvas, clamped to (9, 9)
    let record = circle_brush::draw_circle(100, 100, 5, &mut canvas, red(), 0, false);
    let runs = match &record {
        crate::undo::UndoRecord::Run { runs, .. } => runs,
        _ => unreachable!("draw_circle always produces Run"),
    };
    assert!(!runs.is_empty(), "clamped circle should produce runs");
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
    assert_eq!(
        canvas.pixels[0].pixels[5 * 10 + 5].r(),
        128,
        "r must not be darkend"
    );
}

/// Semi-transparent premultiplied circle line must be stored as-is.
#[test]
fn draw_circle_line_preserves_premultiplied_semi_transparent() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    let semi = Color32::from_rgba_premultiplied(128, 64, 32, 128);
    circle_brush::draw_circle_line(
        BrushStrokeParams {
            start_x: 2,
            start_y: 5,
            end_x: 7,
            end_y: 5,
            canvas: &mut canvas,
            color: semi,
            layer: 0,
            visited: &mut visited,
            stamp: 1,
            alpha_overlay: false,
            drag_processed: &mut drag_processed,
            drag_stamp_value: 0,
        },
        0,
    );
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 2], semi);
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 2].r(), 128);
}
