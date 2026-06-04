//! Tests for custom brush line drawing (`tools::custom_brush`).
//!
//! Covers single-tip placement, drag-line interpolation with spacing,
//! spacing-edge-case clamping, and aspect-ratio preserving scaling.

use eframe::egui::Color32;

use crate::brush_params::BrushStrokeParams;
use crate::tests::common::red;
use crate::tests::common::small_canvas;
use crate::tool_configuration::StampSampling;
use crate::tools::custom_brush;

/// 2x2 white tip for testing.

fn white_tip() -> Vec<Color32> {

    vec![Color32::from_rgba_premultiplied(255, 255, 255, 255); 4]
}

/// 2x4 rectangular white tip for aspect-ratio tests.

fn rect_tip() -> Vec<Color32> {

    vec![Color32::from_rgba_premultiplied(255, 255, 255, 255); 8]
}

/// Place a single brush tip at the center of a canvas (un-tinted, white tip).
#[test]

fn single_tip_at_center() {

    let mut canvas = small_canvas();

    let tip = white_tip();

    let mut visited = vec![0u32; 100];

    let mut drag_processed = vec![0u32; 100];

    custom_brush::draw_custom_brush_line(
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
            drag_stamp_value: 1,
        },
        &tip,
        2,
        2,
        2,
        25,
        false,
        StampSampling::Nearest,
    );

    // Output rect = [4,6] × [4,6] → 2×2 white pixels at canvas center
    for y in 4..=5 {

        for x in 4..=5 {

            let idx = y * 10 + x;

            assert_eq!(
                canvas.pixels[0].pixels[idx],
                Color32::from_rgba_premultiplied(255, 255, 255, 255),
                "single tip at ({x},{y}) should be white (un-tinted)",
            );
        }
    }
}

/// Draw a line with spacing=50; verify stamps are placed along the path.
#[test]

fn line_interpolates_with_spacing() {

    let mut canvas = small_canvas();

    let tip = white_tip();

    let mut visited = vec![0u32; 100];

    let mut drag_processed = vec![0u32; 100];

    custom_brush::draw_custom_brush_line(
        BrushStrokeParams {
            start_x: 0,
            start_y: 0,
            end_x: 10,
            end_y: 10,
            canvas: &mut canvas,
            color: red(),
            layer: 0,
            visited: &mut visited,
            stamp: 1,
            alpha_overlay: false,
            drag_processed: &mut drag_processed,
            drag_stamp_value: 1,
        },
        &tip,
        2,
        2,
        4,
        50,
        false,
        StampSampling::Nearest,
    );

    let center_idx = 5 * 10 + 5;

    assert_eq!(
        canvas.pixels[0].pixels[center_idx].a(),
        255,
        "line interpolation should reach center (5,5)",
    );
}

/// Spacing of 0 should clamp to minimum step of 1 (no panic, still paints).
#[test]

fn spacing_zero_clamps_to_minimum_step() {

    let mut canvas = small_canvas();

    let tip = white_tip();

    let mut visited = vec![0u32; 100];

    let mut drag_processed = vec![0u32; 100];

    custom_brush::draw_custom_brush_line(
        BrushStrokeParams {
            start_x: 2,
            start_y: 2,
            end_x: 2,
            end_y: 2,
            canvas: &mut canvas,
            color: red(),
            layer: 0,
            visited: &mut visited,
            stamp: 1,
            alpha_overlay: false,
            drag_processed: &mut drag_processed,
            drag_stamp_value: 1,
        },
        &tip,
        2,
        2,
        2,
        0,
        false,
        StampSampling::Nearest,
    );

    let painted = canvas.pixels[0].pixels[2 * 10 + 2];

    assert_ne!(painted.a(), 0, "spacing=0 should still paint");
}

/// Rectangular tip paints multiple rows on canvas (aspect-ratio preserved).
#[test]

fn aspect_scaling_rectangular_tip() {

    let mut canvas = small_canvas();

    let tip = rect_tip(); // 2×4
    let mut visited = vec![0u32; 100];

    let mut drag_processed = vec![0u32; 100];

    // radius=2 → output_w=2, output_h = 4 * 2 / 2 = 4
    // Centred at (5,5): output = [4,6] × [3,7]
    custom_brush::draw_custom_brush_line(
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
            drag_stamp_value: 1,
        },
        &tip,
        2,
        4,
        2,
        25,
        false,
        StampSampling::Nearest,
    );

    // Middle pixel should be painted
    let mid = canvas.pixels[0].pixels[5 * 10 + 5];

    assert_eq!(mid.a(), 255, "rect tip should paint at center");

    // Tip is 2 wide × 4 tall, scaled to 2×4 → paints rows 3–6
    // (output_h=4, half_h=2, centre=5 → out_top=3, out_bottom=3+4-1=6)
    let row_top = canvas.pixels[0].pixels[3 * 10 + 5];

    let row_bot = canvas.pixels[0].pixels[6 * 10 + 5];

    assert_eq!(row_top.a(), 255, "rect tip should reach y=3");

    assert_eq!(row_bot.a(), 255, "rect tip should reach y=6");
}
