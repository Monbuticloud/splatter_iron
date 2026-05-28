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
