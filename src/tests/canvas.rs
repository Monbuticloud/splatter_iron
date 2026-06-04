//! Tests for `Canvas` construction, defaults, and `DirtyRect` bookkeeping.
//!
//! Validates canvas creation at various sizes, canonical default output,
//! and the dirty-rect tracking that drives partial GPU uploads.

use eframe::egui::Color32;

use crate::canvas::Canvas;
use crate::canvas::DirtyRect;

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

/// A newly constructed `Canvas` should have an empty `DirtyRectList`.
#[test]

fn canvas_dirty_rect_default_empty() {

    let canvas = Canvas::default();

    assert!(canvas.dirty_rect.is_empty());
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

/// A fresh canvas should request a full blend on first frame.
#[test]

fn new_canvas_requests_full_blend() {

    let canvas = Canvas::new(10, 10);

    assert!(canvas.dirty_rect.needs_reblend());
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

    assert!(
        restored.pixels[0]
            .pixels
            .iter()
            .all(|p| *p == Color32::TRANSPARENT)
    );

    // Skipped fields — should be defaults
    assert!(restored.rendered_layers.is_none());

    assert!(restored.output_rgba.is_empty());

    assert!(restored.dirty_rect.is_empty());
}

/// Multi-layer canvas serialization roundtrip.
#[test]

fn canvas_serde_multi_layer() {

    let mut canvas = Canvas::new(2, 2);

    canvas.pixels.push(crate::canvas::Layer {
        pixels: vec![Color32::RED; 4],
        ..Default::default()
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

// --- DirtyRectList ---

use crate::canvas::DirtyRectList;

/// A new list should be empty.
#[test]

fn dirty_rect_list_empty() {

    let mut list = DirtyRectList::new();

    assert!(list.is_empty());

    assert!(list.take_all().is_empty());
}

/// Adding a rect makes the list non-empty.
#[test]

fn dirty_rect_list_add_single() {

    let mut list = DirtyRectList::new();

    list.add(DirtyRect::new(0, 0, 10, 10));

    assert!(!list.is_empty());

    assert_eq!(list.take_all().len(), 1);
}

/// Two non-overlapping distant rects should stay separate.
#[test]

fn dirty_rect_list_non_overlapping_separate() {

    let mut list = DirtyRectList::new();

    list.add(DirtyRect::new(0, 0, 5, 5));

    list.add(DirtyRect::new(100, 100, 105, 105));

    let rects = list.take_all();

    assert_eq!(rects.len(), 2);
}

/// Two overlapping rects should merge into one.
#[test]

fn dirty_rect_list_overlapping_merges() {

    let mut list = DirtyRectList::new();

    list.add(DirtyRect::new(0, 0, 10, 10));

    list.add(DirtyRect::new(5, 5, 15, 15));

    let rects = list.take_all();

    assert_eq!(rects.len(), 1);

    assert_eq!(rects[0].min_x, 0);

    assert_eq!(rects[0].max_x, 15);
}

/// Two rects within proximity should merge.
#[test]

fn dirty_rect_list_proximity_merges() {

    let mut list = DirtyRectList::new();

    list.add(DirtyRect::new(0, 0, 10, 10));

    // 11px gap — within default proximity of 16, should merge
    list.add(DirtyRect::new(21, 0, 31, 10));

    let rects = list.take_all();

    assert_eq!(rects.len(), 1);

    assert_eq!(rects[0].min_x, 0);

    assert_eq!(rects[0].max_x, 31);
}

/// Two rects beyond proximity should stay separate.
#[test]

fn dirty_rect_list_beyond_proximity_separate() {

    let mut list = DirtyRectList::new();

    list.add(DirtyRect::new(0, 0, 10, 10));

    // 50px gap — beyond proximity, stays separate
    list.add(DirtyRect::new(60, 0, 70, 10));

    let rects = list.take_all();

    assert_eq!(rects.len(), 2);
}

/// Adding many rects beyond the max count triggers merge_all.
#[test]

fn dirty_rect_list_exceeds_max_merges_all() {

    let mut list = DirtyRectList::new();

    for i in 0..9 {

        let x = i as u32 * 200;

        list.add(DirtyRect::new(x, 0, x + 10, 10));
    }

    let rects = list.take_all();

    assert_eq!(rects.len(), 1, "exceeding max should merge all");
}

/// Adding an empty rect should be a no-op.
#[test]

fn dirty_rect_list_add_empty_noop() {

    let mut list = DirtyRectList::new();

    list.add(DirtyRect::empty());

    assert!(list.is_empty());
}

/// Clear should reset the list.
#[test]

fn dirty_rect_list_clear() {

    let mut list = DirtyRectList::new();

    list.add(DirtyRect::new(0, 0, 10, 10));

    assert!(!list.is_empty());

    list.clear();

    assert!(list.is_empty());
}

/// take_all should drain all rects.
#[test]

fn dirty_rect_list_take_all_drains() {

    let mut list = DirtyRectList::new();

    list.add(DirtyRect::new(0, 0, 10, 10));

    assert_eq!(list.take_all().len(), 1);

    assert!(list.is_empty());
}

/// merge_all should combine all rects into one bounding box.
#[test]

fn dirty_rect_list_merge_all() {

    let mut list = DirtyRectList::new();

    list.add(DirtyRect::new(0, 0, 5, 5));

    list.add(DirtyRect::new(100, 100, 105, 105));

    list.merge_all();

    let rects = list.take_all();

    assert_eq!(rects.len(), 1);

    assert_eq!(rects[0].min_x, 0);

    assert_eq!(rects[0].max_y, 105);
}
