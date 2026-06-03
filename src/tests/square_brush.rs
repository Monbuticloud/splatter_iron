//! Tests for rectangular fill (`square_brush::draw_square`) and
//! interpolated stamp-line (`square_brush::draw_square_line`).
//!
//! Covers exact pixel coverage, alpha-overlay strokes, and
//! drag-stamp deduplication for connected rectangular brush strokes.

use std::sync::Arc;

use eframe::egui::Color32;

use crate::brush_params::BrushStrokeParams;
use crate::canvas::Canvas;
use crate::canvas::DirtyRectList;
use crate::canvas::Layer;
use crate::tests::common::red;
use crate::tests::common::small_canvas;
use crate::tools::square_brush;

// --- draw_square ---

/// `draw_square` should fill the specified rectangular region.
#[test]
fn draw_square_fills_region() {
    let mut canvas = small_canvas();
    square_brush::draw_square(1, 1, 4, 4, &mut canvas, red(), 0, false);
    assert_eq!(
        canvas.pixels[0].pixels[1 * 10 + 1],
        red(),
        "top-left of square"
    );
    assert_eq!(
        canvas.pixels[0].pixels[3 * 10 + 3],
        red(),
        "bottom-right of square"
    );
}

/// Pixels outside the square region should remain transparent.
#[test]
fn draw_square_leaves_outside_unchanged() {
    let mut canvas = small_canvas();
    square_brush::draw_square(1, 1, 4, 4, &mut canvas, red(), 0, false);
    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::TRANSPARENT,
        "outside square unchanged"
    );
    assert_eq!(
        canvas.pixels[0].pixels[4 * 10 + 4],
        Color32::TRANSPARENT,
        "past bottom-right unchanged"
    );
}

/// Coordinates exceeding canvas bounds should be clamped.
#[test]
fn draw_square_clamps_to_canvas_bounds() {
    let mut canvas = small_canvas();
    square_brush::draw_square(0, 0, 100, 100, &mut canvas, red(), 0, false);
    // Last pixel should be colored (clamped)
    assert_eq!(canvas.pixels[0].pixels[99], red(), "corner clamped");
    assert_eq!(canvas.pixels[0].pixels[0], red(), "top-left corner colored");
}

/// A zero-area square should not modify any pixels.
#[test]
fn draw_square_zero_area_is_noop() {
    let mut canvas = small_canvas();
    square_brush::draw_square(5, 5, 5, 5, &mut canvas, red(), 0, false);
    assert_eq!(
        canvas.pixels[0].pixels[5 * 10 + 5],
        Color32::TRANSPARENT,
        "zero-area square changes nothing"
    );
}

/// Inverted coordinates (start > end) should produce an empty rect.
#[test]
fn draw_square_inverted_coordinates() {
    let mut canvas = small_canvas();
    square_brush::draw_square(7, 7, 2, 2, &mut canvas, red(), 0, false);
    // start_x > end_x should produce empty rect
    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::TRANSPARENT,
        "inverted coords produce empty rect"
    );
}

/// Drawing on different layers should independently modify each layer.
#[test]
fn draw_square_multi_layer() {
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
        output_rgba: Arc::new(Vec::new()),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
    };
    square_brush::draw_square(0, 0, 5, 5, &mut canvas, red(), 0, false);
    square_brush::draw_square(
        2,
        2,
        7,
        7,
        &mut canvas,
        Color32::from_rgba_premultiplied(0, 0, 255, 255),
        1,
        false,
    );
    assert_eq!(canvas.pixels[0].pixels[0], red(), "layer 0 has red");
    assert_eq!(
        canvas.pixels[1].pixels[3 * 10 + 3],
        Color32::from_rgba_premultiplied(0, 0, 255, 255),
        "layer 1 has blue"
    );
}

// --- draw_square_line ---

/// A horizontal brush line should color pixels at both endrag_processedoints.
#[test]
fn draw_square_line_horizontal() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    square_brush::draw_square_line(
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
        1,
    );
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 1], red(), "start");
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 8], red(), "end");
}

/// A vertical brush line should color pixels at both endrag_processedoints.
#[test]
fn draw_square_line_vertical() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    square_brush::draw_square_line(
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
        1,
    );
    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 5], red(), "start");
    assert_eq!(canvas.pixels[0].pixels[8 * 10 + 5], red(), "end");
}

/// A diagonal brush line should color pixels at both endrag_processedoints.
#[test]
fn draw_square_line_diagonal() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    square_brush::draw_square_line(
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
        1,
    );
    // At least the end points should be colored
    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 1], red(), "start");
    assert_eq!(canvas.pixels[0].pixels[8 * 10 + 8], red(), "end");
}

/// Different stamp values should produce independent brush lines.
#[test]
fn draw_square_line_different_stamps_dont_interfere() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    // First line with stamp 1
    square_brush::draw_square_line(
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
        1,
    );
    // Second line with stamp 2 in a different area
    square_brush::draw_square_line(
        BrushStrokeParams {
            start_x: 6,
            start_y: 6,
            end_x: 8,
            end_y: 6,
            canvas: &mut canvas,
            color: Color32::from_rgba_premultiplied(0, 0, 255, 255),
            layer: 0,
            visited: &mut visited,
            stamp: 2,
            alpha_overlay: false,
            drag_processed: &mut drag_processed,
            drag_stamp_value: 0,
        },
        1,
    );
    // Both stamps should be applied
    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 1], red(), "stamp 1");
    assert_eq!(
        canvas.pixels[0].pixels[6 * 10 + 6],
        Color32::from_rgba_premultiplied(0, 0, 255, 255),
        "stamp 2"
    );
}

/// Brush radius should affect the area covered around the cursor.
#[test]
fn draw_square_line_brush_radius() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    // Brush radius 3 → 7x7 brush, centered at cursor
    square_brush::draw_square_line(
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
    // Pixel at center
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 5], red());
    // Pixel within brush radius
    assert_eq!(canvas.pixels[0].pixels[4 * 10 + 4], red());
    // Pixel at brush edge (within radius 3 from center at 5,5)
    assert_eq!(canvas.pixels[0].pixels[2 * 10 + 2], red(), "brush corner");
    // Pixel outside brush radius
    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 1], Color32::TRANSPARENT);
}

/// Drawing at the canvas edge with a large brush should not panic.
#[test]
fn draw_square_line_clamps_to_canvas() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    square_brush::draw_square_line(
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
    // Should not panic, corner should be colored
    assert_eq!(canvas.pixels[0].pixels[0], red());
}

// --- Alpha overlay ---

/// Alpha overlay mode for `draw_square` should blend instead of overwriting.
#[test]
fn draw_square_alpha_overlay_blends() {
    let mut canvas = small_canvas();
    // Pre-fill pixel with opaque white
    canvas.pixels[0].pixels[0] = Color32::from_rgba_premultiplied(255, 255, 255, 255);
    let semi_red = Color32::from_rgba_premultiplied(128, 0, 0, 128);
    square_brush::draw_square(0, 0, 1, 1, &mut canvas, semi_red, 0, true);
    let blended = canvas.pixels[0].pixels[0];
    // Blended result should differ from both pure white and pure semi_red
    assert_ne!(
        blended,
        Color32::from_rgba_premultiplied(255, 255, 255, 255),
        "alpha overlay changed pixel"
    );
    assert_ne!(blended, semi_red, "alpha overlay blended, not replaced");
    // Result should be fully opaque (white was opaque)
    assert_eq!(blended.a(), 255, "alpha overlay result is opaque");
}

/// Alpha overlay mode for `draw_square_line` should blend instead of overwriting.
#[test]
fn draw_square_line_alpha_overlay_blends() {
    let mut canvas = small_canvas();
    canvas.pixels[0].pixels[5 * 10 + 1] = Color32::from_rgba_premultiplied(255, 255, 255, 255);
    let mut visited = vec![0u32; 100];
    let mut drag_processed = vec![0u32; 100];
    let semi_red = Color32::from_rgba_premultiplied(128, 0, 0, 128);
    square_brush::draw_square_line(
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
        1,
    );
    let blended = canvas.pixels[0].pixels[5 * 10 + 1];
    assert_ne!(
        blended,
        Color32::from_rgba_premultiplied(255, 255, 255, 255),
        "alpha overlay changed pixel"
    );
    assert_ne!(blended, semi_red, "alpha overlay blended");
}

// --------------------------------------------------
//  Regression: semi-transparent premultiplied color
// --------------------------------------------------

/// Semi-transparent premultiplied color must be stored as-is (not double-premultiplied).
#[test]
fn draw_square_preserves_premultiplied_semi_transparent() {
    let mut canvas = small_canvas();
    let semi = Color32::from_rgba_premultiplied(128, 64, 32, 128);
    square_brush::draw_square(0, 0, 5, 5, &mut canvas, semi, 0, false);
    assert_eq!(
        canvas.pixels[0].pixels[0], semi,
        "pixel should store exact premultiplied color"
    );
    assert_eq!(canvas.pixels[0].pixels[0].r(), 128, "r must not be darkend");
}

/// Semi-transparent premultiplied line must be stored as-is.
#[test]
fn draw_square_line_preserves_premultiplied_semi_transparent() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    let mut drag_processed = Vec::new();
    let semi = Color32::from_rgba_premultiplied(128, 64, 32, 128);
    square_brush::draw_square_line(
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
        1,
    );
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 2], semi);
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 2].r(), 128);
}
