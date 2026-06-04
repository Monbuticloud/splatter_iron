//! Tests for scanline flood-fill (`bucket_fill::draw_bucket_fill`).
//!
//! Covers filling from single-pixel and multi-pixel seeds, edge-of-canvas
//! seeds, fully-saturated canvases, and alpha-overlay mode.

use std::sync::Arc;

use eframe::egui::Color32;

use crate::canvas::Canvas;
use crate::canvas::DirtyRectList;
use crate::canvas::Layer;
use crate::tests::common::blue;
use crate::tests::common::red;
use crate::tests::common::small_canvas;
use crate::tools::bucket_fill;

/// Build a canvas with a pre-drawn 2×2 red square at (1,1)–(3,3).

fn canvas_with_red_square() -> Canvas {

    let mut canvas = small_canvas();

    crate::tools::square_brush::draw_square(1, 1, 4, 4, &mut canvas, red(), 0, false);

    canvas
}

// --- draw_bucket_fill ---

/// Fills all contiguous same-color pixels from the seed point.
#[test]

fn bucket_fill_fills_contiguous_region() {

    let mut canvas = canvas_with_red_square();

    // The red square at (1..4, 1..4) — fill it with blue
    bucket_fill::draw_bucket_fill(2, 2, &mut canvas, blue(), 0, false);

    // Center of the square should be blue
    assert_eq!(
        canvas.pixels[0].pixels[2 * 10 + 2],
        blue(),
        "center of filled region"
    );

    // Edge of the square should be blue
    assert_eq!(
        canvas.pixels[0].pixels[1 * 10 + 1],
        blue(),
        "edge of filled region"
    );
}

/// Does not leak to disconnected same-color pixels outside the contiguous region.
#[test]

fn bucket_fill_does_not_leak() {

    let mut canvas = canvas_with_red_square();

    // Fill at (1,1) — should only fill the red square
    bucket_fill::draw_bucket_fill(1, 1, &mut canvas, blue(), 0, false);

    // Pixels outside the square should remain transparent
    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::TRANSPARENT,
        "outside region unchanged"
    );

    assert_eq!(
        canvas.pixels[0].pixels[4 * 10 + 4],
        Color32::TRANSPARENT,
        "outside region unchanged"
    );
}

/// No-op when target color already equals fill color.
#[test]

fn bucket_fill_same_color_noop() {

    let mut canvas = canvas_with_red_square();

    // Fill red with red — should be a no-op
    bucket_fill::draw_bucket_fill(2, 2, &mut canvas, red(), 0, false);

    // Pixels should still be red
    assert_eq!(canvas.pixels[0].pixels[2 * 10 + 2], red());

    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 1], red());

    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::TRANSPARENT,
        "outside still transparent"
    );
}

/// Multi-layer isolation: fill affects only the target layer.
#[test]

fn bucket_fill_multi_layer() {

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

    // Draw red square on layer 0, blue on layer 1
    crate::tools::square_brush::draw_square(1, 1, 4, 4, &mut canvas, red(), 0, false);

    crate::tools::square_brush::draw_square(2, 2, 5, 5, &mut canvas, blue(), 1, false);

    // Fill layer 0 at (2,2) with blue — only layer 0 should change
    bucket_fill::draw_bucket_fill(2, 2, &mut canvas, blue(), 0, false);

    // Layer 0 at (2,2) should now be blue
    assert_eq!(
        canvas.pixels[0].pixels[2 * 10 + 2],
        blue(),
        "layer 0 filled"
    );

    // Layer 1 should be unchanged (still blue at (2,2))
    assert_eq!(
        canvas.pixels[1].pixels[2 * 10 + 2],
        blue(),
        "layer 1 unchanged"
    );

    // Layer 0 outside should still be transparent
    assert_eq!(
        canvas.pixels[0].pixels[0],
        Color32::TRANSPARENT,
        "layer 0 outside unchanged"
    );
}

/// Fills correctly from the corner seed (0,0).
#[test]

fn bucket_fill_corner_seed() {

    let mut canvas = small_canvas();

    // Fill entire canvas with red using square brush
    crate::tools::square_brush::draw_square(0, 0, 10, 10, &mut canvas, red(), 0, false);

    // Now fill from corner with blue
    bucket_fill::draw_bucket_fill(0, 0, &mut canvas, blue(), 0, false);

    // Entire canvas should be blue
    assert_eq!(canvas.pixels[0].pixels[0], blue(), "corner pixel");

    assert_eq!(
        canvas.pixels[0].pixels[9 * 10 + 9],
        blue(),
        "opposite corner"
    );
}

/// Returns a non-empty undo record with run segments.
#[test]

fn bucket_fill_returns_undo() {

    let mut canvas = canvas_with_red_square();

    let record = bucket_fill::draw_bucket_fill(2, 2, &mut canvas, blue(), 0, false);

    let runs = match &record {
        crate::undo::UndoRecord::Run { runs, .. } => runs,
        _ => unreachable!("bucket_fill always produces Run"),
    };

    assert!(!runs.is_empty(), "undo should contain run segments");
}

// --------------------------------------------------
//  Regression: semi-transparent premultiplied color
// --------------------------------------------------

/// Semi-transparent premultiplied fill must be stored as-is (not double-premultiplied).
#[test]

fn bucket_fill_preserves_premultiplied_semi_transparent() {

    let mut canvas = small_canvas();

    let semi = Color32::from_rgba_premultiplied(128, 64, 32, 128);

    // Fill entire canvas with semi-transparent color
    bucket_fill::draw_bucket_fill(0, 0, &mut canvas, semi, 0, false);

    assert_eq!(
        canvas.pixels[0].pixels[0], semi,
        "pixel should store exact premultiplied color"
    );

    assert_eq!(canvas.pixels[0].pixels[0].r(), 128, "r must not be darkend");

    assert_eq!(
        canvas.pixels[0].pixels[9 * 10 + 9],
        semi,
        "far corner also preserved"
    );
}

// --- Alpha overlay ---

/// Alpha overlay mode should blend the fill color over the existing pixels.
#[test]

fn bucket_fill_alpha_overlay_blends() {

    let mut canvas = small_canvas();

    // Pre-fill with opaque white
    crate::tools::square_brush::draw_square(
        0,
        0,
        10,
        10,
        &mut canvas,
        Color32::from_rgba_premultiplied(255, 255, 255, 255),
        0,
        false,
    );

    let semi_red = Color32::from_rgba_premultiplied(128, 0, 0, 128);

    bucket_fill::draw_bucket_fill(0, 0, &mut canvas, semi_red, 0, true);

    let blended = canvas.pixels[0].pixels[0];

    // Blended result should be different from both pure white and pure semi_red
    assert_ne!(
        blended,
        Color32::from_rgba_premultiplied(255, 255, 255, 255),
        "alpha overlay changed pixel"
    );

    assert_ne!(blended, semi_red, "alpha overlay blended, not replaced");

    // Alpha should be fully opaque (white was opaque, semi-red adds to it)
    assert_eq!(blended.a(), 255, "alpha overlay result is opaque");
}

// --- Seed outside canvas bounds ---

/// A fill seed outside canvas bounds should clamp and fill from the edge.
#[test]

fn bucket_fill_seed_outside_bounds() {

    let mut canvas = small_canvas();

    // Fill the canvas with red first
    crate::tools::square_brush::draw_square(0, 0, 10, 10, &mut canvas, red(), 0, false);

    // Seed at (100, 100) — outside canvas — should clamp to (9, 9)
    bucket_fill::draw_bucket_fill(100, 100, &mut canvas, blue(), 0, false);

    // The clamped seed should find the red region and fill it
    assert_eq!(
        canvas.pixels[0].pixels[99],
        blue(),
        "far corner filled after clamped seed"
    );
}
