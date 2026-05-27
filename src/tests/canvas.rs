use eframe::egui::Color32;

use crate::canvas::Canvas;

// --- Canvas defaults ---

/// The default canvas should be 2000×1500.
#[test]
fn default_canvas_size() {
    let canvas = Canvas::default();
    assert_eq!(canvas.width, 2000);
    assert_eq!(canvas.height, 1500);
}

/// The default canvas should have one fully transparent layer.
#[test]
fn default_canvas_has_one_transparent_layer() {
    let canvas = Canvas::default();
    assert_eq!(canvas.pixels.len(), 1);
    assert_eq!(canvas.pixels[0].pixels.len(), 2000 * 1500);
    assert_eq!(canvas.pixels[0].pixels[0], Color32::TRANSPARENT);
}
