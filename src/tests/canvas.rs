use eframe::egui::Color32;

use crate::canvas::{self, Canvas, Layer};
use crate::undo::UndoRecord;

/// Build a 10×10 single-layer transparent canvas for use in tests.
fn small_canvas() -> Canvas {
    Canvas {
        pixels: vec![Layer {
            pixels: vec![Color32::TRANSPARENT; 100],
        }],
        height: 10,
        width: 10,
        output_rgba: Vec::new(),
        rendered_layers: None,
        render_next_frame: false,
    }
}

/// Shortcut for a fully opaque red colour in premultiplied format.
fn red() -> Color32 {
    Color32::from_rgba_premultiplied(255, 0, 0, 255)
}

// --- draw_square ---

/// `draw_square` should fill the specified rectangular region.
#[test]
fn draw_square_fills_region() {
    let mut canvas = small_canvas();
    canvas::draw_square(1, 1, 4, 4, &mut canvas, red(), 0);
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
    canvas::draw_square(1, 1, 4, 4, &mut canvas, red(), 0);
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
    canvas::draw_square(0, 0, 100, 100, &mut canvas, red(), 0);
    // Last pixel should be colored (clamped)
    assert_eq!(canvas.pixels[0].pixels[99], red(), "corner clamped");
    assert_eq!(
        canvas.pixels[0].pixels[0],
        red(),
        "top-left corner colored"
    );
}

/// A zero-area square should not modify any pixels.
#[test]
fn draw_square_zero_area_is_noop() {
    let mut canvas = small_canvas();
    canvas::draw_square(5, 5, 5, 5, &mut canvas, red(), 0);
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
    canvas::draw_square(7, 7, 2, 2, &mut canvas, red(), 0);
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
            Layer { pixels: vec![Color32::TRANSPARENT; 100] },
            Layer { pixels: vec![Color32::TRANSPARENT; 100] },
        ],
        height: 10,
        width: 10,
        output_rgba: Vec::new(),
        rendered_layers: None,
        render_next_frame: false,
    };
    canvas::draw_square(0, 0, 5, 5, &mut canvas, red(), 0);
    canvas::draw_square(
        2,
        2,
        7,
        7,
        &mut canvas,
        Color32::from_rgba_premultiplied(0, 0, 255, 255),
        1,
    );
    assert_eq!(canvas.pixels[0].pixels[0], red(), "layer 0 has red");
    assert_eq!(
        canvas.pixels[1].pixels[3 * 10 + 3],
        Color32::from_rgba_premultiplied(0, 0, 255, 255),
        "layer 1 has blue"
    );
}

// --- draw_square_line ---

/// A horizontal brush line should colour pixels at both endpoints.
#[test]
fn draw_square_line_horizontal() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    canvas::draw_square_line(1, 5, 8, 5, 1, &mut canvas, red(), 0, &mut visited, 1);
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 1], red(), "start");
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 8], red(), "end");
}

#[test]
fn draw_square_line_vertical() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    canvas::draw_square_line(5, 1, 5, 8, 1, &mut canvas, red(), 0, &mut visited, 1);
    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 5], red(), "start");
    assert_eq!(canvas.pixels[0].pixels[8 * 10 + 5], red(), "end");
}

#[test]
fn draw_square_line_diagonal() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    canvas::draw_square_line(1, 1, 8, 8, 1, &mut canvas, red(), 0, &mut visited, 1);
    // At least the end points should be colored
    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 1], red(), "start");
    assert_eq!(canvas.pixels[0].pixels[8 * 10 + 8], red(), "end");
}

#[test]
fn draw_square_line_different_stamps_dont_interfere() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    // First line with stamp 1
    canvas::draw_square_line(1, 1, 3, 1, 1, &mut canvas, red(), 0, &mut visited, 1);
    // Second line with stamp 2 in a different area
    canvas::draw_square_line(
        6,
        6,
        8,
        6,
        1,
        &mut canvas,
        Color32::from_rgba_premultiplied(0, 0, 255, 255),
        0,
        &mut visited,
        2,
    );
    // Both stamps should be applied
    assert_eq!(canvas.pixels[0].pixels[1 * 10 + 1], red(), "stamp 1");
    assert_eq!(
        canvas.pixels[0].pixels[6 * 10 + 6],
        Color32::from_rgba_premultiplied(0, 0, 255, 255),
        "stamp 2"
    );
}

#[test]
fn draw_square_line_brush_radius() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    // Brush radius 3 → 7x7 brush, centered at cursor
    canvas::draw_square_line(5, 5, 5, 5, 3, &mut canvas, red(), 0, &mut visited, 1);
    // Pixel at center
    assert_eq!(canvas.pixels[0].pixels[5 * 10 + 5], red());
    // Pixel within brush radius
    assert_eq!(canvas.pixels[0].pixels[4 * 10 + 4], red());
    // Pixel outside brush radius
    assert_eq!(
        canvas.pixels[0].pixels[2 * 10 + 2],
        Color32::TRANSPARENT
    );
}

#[test]
fn draw_square_line_clamps_to_canvas() {
    let mut canvas = small_canvas();
    let mut visited = vec![0u32; 100];
    // Single point at corner with large brush
    canvas::draw_square_line(0, 0, 0, 0, 5, &mut canvas, red(), 0, &mut visited, 1);
    // Should not panic, corner should be colored
    assert_eq!(canvas.pixels[0].pixels[0], red());
}

// --- Canvas defaults ---

#[test]
fn default_canvas_size() {
    let canvas = Canvas::default();
    assert_eq!(canvas.width, 2000);
    assert_eq!(canvas.height, 1500);
}

#[test]
fn default_canvas_has_one_transparent_layer() {
    let canvas = Canvas::default();
    assert_eq!(canvas.pixels.len(), 1);
    assert_eq!(canvas.pixels[0].pixels.len(), 2000 * 1500);
    assert_eq!(canvas.pixels[0].pixels[0], Color32::TRANSPARENT);
}
