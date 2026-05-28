//! Tests for stamp brush (`stamp_brush::draw_stamp_line`).
//!
//! Covers single-stamp placement, scaling, tinting, alpha overlay,
//! drag-line interpolation, and edge clamping.

use eframe::egui::Color32;

use crate::stamp_library::StampSampling;
use crate::tests::common::{ red, blue, small_canvas };
use crate::tools::stamp_brush;

/// Build a 2×2 stamp: top-left red, top-right green, bottom-left blue,
/// bottom-right white.
fn solid_stamp() -> (Vec<Color32>, u32, u32) {
    let green = Color32::from_rgba_premultiplied(0, 255, 0, 255);
    let white = Color32::from_rgba_premultiplied(255, 255, 255, 255);
    let pixels = vec![red(), green, blue(), white];
    (pixels, 2, 2)
}

/// Single stamp at centre should place stamp pixels in the correct positions.
#[test]
fn single_stamp_at_center() {
    let mut canvas = small_canvas();
    let (stamp, w, h) = solid_stamp();
    let mut drag_processed = vec![0u32; 100];

    // radius=2 → output 2×2 centred at (5,5) → covers (4..5, 4..5)
    stamp_brush::draw_stamp_line(
        5, 5, 5, 5, &stamp, w, h, 2, &mut canvas, red(), 0, false, false,
        StampSampling::Nearest,
        &mut drag_processed, 1,
    );

    let pixels = &canvas.pixels[0].pixels;
    // output (4,4) → src (0,0) = red
    assert_eq!(pixels[4 * 10 + 4], red(), "top-left: src (0,0)");
    // output (5,4) → src (1,0) = green
    assert_eq!(pixels[4 * 10 + 5], Color32::from_rgba_premultiplied(0, 255, 0, 255), "top-right: src (1,0)");
    // output (4,5) → src (0,1) = blue
    assert_eq!(pixels[5 * 10 + 4], blue(), "bottom-left: src (0,1)");
    // output (5,5) → src (1,1) = white
    assert_eq!(pixels[5 * 10 + 5], Color32::from_rgba_premultiplied(255, 255, 255, 255), "bottom-right: src (1,1)");
}

/// Stamp with radius 0 should produce a 1×1 output (minimum size)
/// mapping to source (0,0).
#[test]
fn stamp_minimum_radius() {
    let mut canvas = small_canvas();
    let (stamp, w, h) = solid_stamp();
    let mut drag_processed = vec![0u32; 100];

    stamp_brush::draw_stamp_line(
        5, 5, 5, 5, &stamp, w, h, 0, &mut canvas, red(), 0, false, false,
        StampSampling::Nearest,
        &mut drag_processed, 1,
    );

    // radius 0 → output 1×1 → only pixel (5,5) → nearest src (0,0) = red
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 5], red(), "radius 0 → 1×1 src (0,0)");
}

/// Tinted mode should multiply stamp pixels by the tint colour.
#[test]
fn tinted_stamp_applies_color() {
    let mut canvas = small_canvas();
    let (stamp, w, h) = solid_stamp();
    let mut drag_processed = vec![0u32; 100];
    let tint = Color32::from_rgba_premultiplied(128, 128, 128, 255);

    stamp_brush::draw_stamp_line(
        5, 5, 5, 5, &stamp, w, h, 2, &mut canvas, tint, 0, false, true,
        StampSampling::Nearest,
        &mut drag_processed, 1,
    );

    // Red * 128/255 → (128, 0, 0, 255)
    let expected = Color32::from_rgba_premultiplied(128, 0, 0, 255);
    assert_eq!(
        canvas.pixels[0].pixels[4 * 10 + 4],
        expected,
        "tinted red pixel",
    );
}

/// Alpha-overlay blends stamp over existing background.
#[test]
fn alpha_overlay_blends_stamp() {
    let mut canvas = small_canvas();
    // Fill pixel (4,4) with opaque blue
    canvas.pixels[0].pixels[4 * 10 + 4] = blue();

    // Use a semi-transparent red stamp pixel for visible blending
    let semi_red = Color32::from_rgba_premultiplied(128, 0, 0, 128);
    let stamp = vec![semi_red];
    let mut drag_processed = vec![0u32; 100];

    stamp_brush::draw_stamp_line(
        5, 5, 5, 5, &stamp, 1, 1, 2, &mut canvas, red(), 0, true, false,
        StampSampling::Nearest,
        &mut drag_processed, 1,
    );

    let blended = canvas.pixels[0].pixels[4 * 10 + 4];
    assert_ne!(blended, blue(), "alpha overlay should blend with background");
    assert_ne!(blended, semi_red, "alpha overlay should alter source");
}

/// Stamp at the corner should clamp without panicking and place
/// the visible portion of the image.
#[test]
fn stamp_clamps_to_canvas_edge() {
    let mut canvas = small_canvas();
    let (stamp, w, h) = solid_stamp();
    let mut drag_processed = vec![0u32; 100];

    // Centre at (0,0), radius=4 → output 4×4, half=2
    // out_left = -2, out_top = -2 → clamped to (0,0)
    // Only bottom-right quarter is visible
    stamp_brush::draw_stamp_line(
        0, 0, 0, 0, &stamp, w, h, 4, &mut canvas, red(), 0, false, false,
        StampSampling::Nearest,
        &mut drag_processed, 1,
    );

    // Pixel (0,0) → src_x = (0-(-2))*2/4 = 1, src_y = (0-(-2))*2/4 = 1 → src (1,1) = white
    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::from_rgba_premultiplied(255, 255, 255, 255),
        "corner stamp maps visible region to correct source pixel",
    );
}

/// Drag line should place stamps along the interpolated path.
#[test]
fn stamp_line_interpolates() {
    let mut canvas = small_canvas();
    let (stamp, w, h) = solid_stamp();
    let mut drag_processed = vec![0u32; 100];

    // Drag from (2,2) to (7,7), radius=4 → output 4×4, step=2
    // Multiple stamps should be placed along the diagonal
    stamp_brush::draw_stamp_line(
        2, 2, 7, 7, &stamp, w, h, 4, &mut canvas, red(), 0, false, false,
        StampSampling::Nearest,
        &mut drag_processed, 1,
    );

    // A stamp should be placed at or near (7,7), making pixel (7,7) non-transparent
    assert_ne!(
        canvas.pixels[0].pixels[7 * 10 + 7],
        Color32::TRANSPARENT,
        "stamp line should reach end point",
    );
}

/// Oversized stamp (radius >> canvas) should clamp gracefully
/// and fill visible area.
#[test]
fn oversized_stamp_clamps() {
    let mut canvas = small_canvas();
    let (stamp, w, h) = solid_stamp();
    let mut drag_processed = vec![0u32; 100];

    stamp_brush::draw_stamp_line(
        5, 5, 5, 5, &stamp, w, h, 100, &mut canvas, red(), 0, false, false,
        StampSampling::Nearest,
        &mut drag_processed, 1,
    );

    // output 100×100 centred at (5,5) → entire 10×10 canvas covered
    // All four corners should have stamp pixels
    assert_ne!(canvas.pixels[0].pixels[0], Color32::TRANSPARENT, "top-left covered");
    assert_ne!(canvas.pixels[0].pixels[9], Color32::TRANSPARENT, "top-right covered");
    assert_ne!(canvas.pixels[0].pixels[90], Color32::TRANSPARENT, "bottom-left covered");
    assert_ne!(canvas.pixels[0].pixels[99], Color32::TRANSPARENT, "bottom-right covered");
}

/// Undo record should contain correct runs.
#[test]
fn stamp_produces_valid_undo_record() {
    let mut canvas = small_canvas();
    let (stamp, w, h) = solid_stamp();
    let mut drag_processed = vec![0u32; 100];

    let record = stamp_brush::draw_stamp_line(
        5, 5, 5, 5, &stamp, w, h, 2, &mut canvas, red(), 0, false, false,
        StampSampling::Nearest,
        &mut drag_processed, 1,
    );

    let crate::undo::UndoRecord::Run { layer_index, runs, .. } = &record;
    assert_eq!(*layer_index, 0, "undo record should target layer 0");
    assert!(!runs.is_empty(), "undo record should have runs");
}

/// Non-overlay stamp should leave surrounding pixels unchanged.
#[test]
fn stamp_does_not_affect_outside() {
    let mut canvas = small_canvas();
    let (stamp, w, h) = solid_stamp();
    let mut drag_processed = vec![0u32; 100];

    stamp_brush::draw_stamp_line(
        5, 5, 5, 5, &stamp, w, h, 2, &mut canvas, red(), 0, false, false,
        StampSampling::Nearest,
        &mut drag_processed, 1,
    );

    // Pixel far from stamp should remain transparent
    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::TRANSPARENT,
        "pixel outside stamp bounds unchanged",
    );
}

/// Rectangular stamp (non-square) should preserve aspect ratio when placed.
#[test]
fn stamp_rectangular_aspect() {
    let mut canvas = small_canvas();
    // 4×1 stamp: red, green, blue, white
    let stamp = vec![
        red(),
        Color32::from_rgba_premultiplied(0, 255, 0, 255),
        blue(),
        Color32::from_rgba_premultiplied(255, 255, 255, 255),
    ];
    let mut drag_processed = vec![0u32; 100];

    // radius=4 → output 4×1 (preserves 4:1 aspect)
    stamp_brush::draw_stamp_line(
        5, 5, 5, 5, &stamp, 4, 1, 4, &mut canvas, red(), 0, false, false,
        StampSampling::Nearest,
        &mut drag_processed, 1,
    );

    let pixels = &canvas.pixels[0].pixels;
    // Source (0,0) → output (3,5) = red
    assert_eq!(pixels[5 * 10 + 3], red(), "rect stamp left pixel");
    // Source (3,0) → output (6,5) = white
    assert_eq!(
        pixels[5 * 10 + 6],
        Color32::from_rgba_premultiplied(255, 255, 255, 255),
        "rect stamp right pixel"
    );
}

/// Stamp fully off-screen should be a no-op and produce an empty undo record.
#[test]
fn stamp_fully_off_screen_noop() {
    let mut canvas = small_canvas();
    let (stamp, w, h) = solid_stamp();
    let mut drag_processed = vec![0u32; 100];

    // Center at (100, 100) — far outside the 10×10 canvas
    stamp_brush::draw_stamp_line(
        100, 100, 100, 100, &stamp, w, h, 2, &mut canvas, red(), 0, false, false,
        StampSampling::Nearest,
        &mut drag_processed, 1,
    );

    // Canvas should remain fully transparent
    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::TRANSPARENT,
        "off-screen stamp leaves canvas unchanged",
    );
}
