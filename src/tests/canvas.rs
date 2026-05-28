//! Tests for `Canvas` construction, defaults, and `DirtyRect` bookkeeping.
//!
//! Validates canvas creation at various sizes, canonical default output,
//! and the dirty-rect tracking that drives partial GPU uploads.

use eframe::egui::Color32;

use crate::canvas::{ Canvas, DirtyRect };

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

// --- DirtyRect ---

/// An empty rect should report `is_empty`.
#[test]
fn dirty_rect_empty() {
    let rect = DirtyRect::empty();
    assert!(rect.is_empty());
    assert_eq!(rect.width(), 0);
    assert_eq!(rect.height(), 0);
}

/// Extending an empty rect with a point should produce a 1×1 rect.
#[test]
fn dirty_rect_extend_from_empty() {
    let mut rect = DirtyRect::empty();
    rect.extend(5, 7);
    assert!(!rect.is_empty());
    assert_eq!(rect.min_x, 5);
    assert_eq!(rect.min_y, 7);
    assert_eq!(rect.max_x, 5);
    assert_eq!(rect.max_y, 7);
    assert_eq!(rect.width(), 1);
    assert_eq!(rect.height(), 1);
}

/// Multiple extends should expand the bounding box.
#[test]
fn dirty_rect_extend_multiple() {
    let mut rect = DirtyRect::empty();
    rect.extend(10, 20);
    rect.extend(5, 30);
    rect.extend(15, 25);
    assert_eq!(rect.min_x, 5);
    assert_eq!(rect.min_y, 20);
    assert_eq!(rect.max_x, 15);
    assert_eq!(rect.max_y, 30);
}

/// Union of two rects should produce a rect covering both.
#[test]
fn dirty_rect_union() {
    let rect_a = DirtyRect::new(0, 0, 10, 10);
    let rect_b = DirtyRect::new(5, 5, 20, 20);
    let union_rect = rect_a.union(&rect_b);
    assert_eq!(union_rect.min_x, 0);
    assert_eq!(union_rect.min_y, 0);
    assert_eq!(union_rect.max_x, 20);
    assert_eq!(union_rect.max_y, 20);
    assert_eq!(union_rect.width(), 21);
    assert_eq!(union_rect.height(), 21);
}

/// A newly constructed `Canvas` should have `dirty_rect: None`.
#[test]
fn canvas_dirty_rect_default_none() {
    let canvas = Canvas::default();
    assert!(canvas.dirty_rect.is_none());
}

// --- Canvas::new ---

/// `Canvas::new(width, height)` should set dimensions and create one transparent layer.
#[test]
fn canvas_new_sets_dimensions() {
    let canvas = Canvas::new(42, 24);
    assert_eq!(canvas.width, 42);
    assert_eq!(canvas.height, 24);
    assert_eq!(canvas.pixels.len(), 1);
    assert_eq!(canvas.pixels[0].pixels.len(), 42 * 24);
    assert_eq!(canvas.pixels[0].pixels[0], Color32::TRANSPARENT);
}

/// The default canvas should have `render_next_frame` set to `true`.
#[test]
fn default_render_next_frame_is_true() {
    let canvas = Canvas::default();
    assert!(canvas.render_next_frame);
}

/// `Canvas::new` should also set `render_next_frame` to `true`.
#[test]
fn canvas_new_render_next_frame_is_true() {
    let canvas = Canvas::new(10, 10);
    assert!(canvas.render_next_frame);
}

// --- Serde round-trip ---

/// Canvas serialization/deserialization should preserve dimensions and layers
/// while skipping GPU/texture fields.
#[test]
fn canvas_serde_roundtrip() {
    let original = Canvas::new(5, 3);
    let json = serde_json::to_string(&original).expect("serialize");
    let restored: Canvas = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(restored.width, 5);
    assert_eq!(restored.height, 3);
    assert_eq!(restored.pixels.len(), 1);
    assert_eq!(
        restored.pixels[0].pixels.len(),
        5 * 3,
        "pixel count preserved"
    );
    assert!(restored.pixels[0].pixels.iter().all(|p| *p == Color32::TRANSPARENT));
    assert_eq!(restored.render_next_frame, original.render_next_frame);

    // Skipped fields — should be defaults
    assert!(restored.rendered_layers.is_none());
    assert!(restored.output_rgba.is_empty());
    assert!(restored.dirty_rect.is_none());
}

/// Multi-layer canvas serialization roundtrip.
#[test]
fn canvas_serde_multi_layer() {
    let mut canvas = Canvas::new(2, 2);
    canvas.pixels.push(crate::canvas::Layer {
        pixels: vec![Color32::RED; 4],
    });

    let json = serde_json::to_string(&canvas).expect("serialize");
    let restored: Canvas = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(restored.pixels.len(), 2);
    assert_eq!(restored.pixels[1].pixels[0], Color32::RED);
}

// --- DirtyRect edge cases ---

/// A DirtyRect with inverted min/max values is empty.
#[test]
fn dirty_rect_inverted_is_empty() {
    let rect = DirtyRect::new(10, 10, 5, 5);
    assert!(rect.is_empty());
    assert_eq!(rect.width(), 0);
    assert_eq!(rect.height(), 0);
}

/// Extending an already-inverted rect covers the new point to original max.
#[test]
fn dirty_rect_extend_fixes_inverted() {
    let mut rect = DirtyRect::new(10, 10, 5, 5);
    assert!(rect.is_empty());
    rect.extend(7, 7);
    assert!(!rect.is_empty());
    assert_eq!(rect.min_x, 7);
    assert_eq!(rect.max_x, 7, "max_x was 5 and extends to 7");
    assert_eq!(rect.width(), 1);
}

/// DirtyRect zero-width rect after union.
#[test]
fn dirty_rect_union_with_empty() {
    let rect_a = DirtyRect::new(5, 5, 5, 5);
    let rect_b = DirtyRect::empty();
    let union_rect = rect_a.union(&rect_b);
    // union with empty: empty's min is MAX, max is 0, so min picks 5, max picks 5
    assert_eq!(union_rect.min_x, 5);
    assert_eq!(union_rect.max_x, 5);
    assert_eq!(union_rect.width(), 1);
}

/// DirtyRect single pixel.
#[test]
fn dirty_rect_single_pixel() {
    let rect = DirtyRect::new(3, 7, 3, 7);
    assert!(!rect.is_empty());
    assert_eq!(rect.width(), 1);
    assert_eq!(rect.height(), 1);
}

/// DirtyRect default is zero-initialized (non-empty, covers one pixel at origin).
#[test]
fn dirty_rect_default_covers_origin() {
    let rect = DirtyRect::default();
    assert!(!rect.is_empty(), "default is 0,0,0,0 — covers origin");
    assert_eq!(rect.min_x, 0);
    assert_eq!(rect.max_x, 0);
    assert_eq!(rect.min_y, 0);
    assert_eq!(rect.max_y, 0);
    assert_eq!(rect.width(), 1);
    assert_eq!(rect.height(), 1);
}
