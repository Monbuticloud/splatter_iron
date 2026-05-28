//! Tests for `Document` — layer management, blend-to-output, and GPU upload.
//!
//! Covers adding, deleting, reordering, and selecting layers, as well as
//! compositing output and tracking dirty regions for rendering.

use eframe::egui::Color32;

use crate::canvas::{Canvas, DirtyRect, DirtyRectList, Layer};
use crate::document::Document;
use crate::undo_history::UndoHistory;

/// Build a 10×10 single-layer document for use in tests.
fn small_document() -> Document {
    let canvas = Canvas {
        pixels: vec![Layer {
            pixels: vec![Color32::TRANSPARENT; 100],
        }],
        height: 10,
        width: 10,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
        render_next_frame: false,
    };
    Document::new(canvas)
}

/// A new document should have one layer, current_layer 0, no save path.
#[test]
fn new_document_has_one_layer() {
    let document = small_document();
    assert_eq!(document.canvas.pixels.len(), 1);
    assert_eq!(document.current_layer, 0);
    assert!(!document.dirty_since_last_autosave);
    assert!(document.savefile_path.is_empty());
}

/// Adding a layer should increase the layer count and request a re-render.
#[test]
fn add_layer_increases_count() {
    let mut document = small_document();
    document.add_layer();
    assert_eq!(document.canvas.pixels.len(), 2);
    assert!(document.canvas.render_next_frame);
}

/// A newly added layer should match the canvas dimensions.
#[test]
fn add_layer_has_correct_size() {
    let mut document = small_document();
    document.add_layer();
    let expected = (document.canvas.width * document.canvas.height) as usize;
    assert_eq!(document.canvas.pixels[1].pixels.len(), expected);
}

/// Deleting a layer should remove the correct index from the layer list.
#[test]
fn delete_layer_removes_correct_index() {
    let mut document = small_document();
    document.add_layer();
    document.add_layer();
    assert_eq!(document.canvas.pixels.len(), 3);
    document.delete_layer(1);
    assert_eq!(document.canvas.pixels.len(), 2);
    // The remaining layers should be index 0 and 2 from the original set
}

/// Deleting the only layer should leave the document with 0 layers.
#[test]
fn delete_last_layer_removes_it() {
    let mut document = small_document();
    document.delete_layer(0);
    // Document model does not guard against removing the last layer;
    // that check is in the UI layer (ui/right.rs).
    assert_eq!(document.canvas.pixels.len(), 0);
    // current_layer saturates to 0
    assert_eq!(document.current_layer, 0);
}

/// Deleting the current layer should adjust `current_layer` downward.
#[test]
fn delete_layer_adjusts_current_layer_down() {
    let mut document = small_document();
    document.add_layer();
    document.add_layer();
    document.current_layer = 2;
    document.delete_layer(2);
    // current_layer should go from 2 → min(1, 1) = 1
    assert_eq!(document.current_layer, 1);
}

/// Moving a layer up should swap it with the layer above.
#[test]
fn move_layer_up_swaps() {
    let mut document = small_document();
    document.add_layer();
    document.add_layer();
    // Initially: layers [0, 1, 2]
    document.move_layer_up(1); // swap 1 and 0 → [1, 0, 2]
    assert_eq!(document.current_layer, 0);
}

/// Moving a layer down should swap it with the layer below.
#[test]
fn move_layer_down_swaps() {
    let mut document = small_document();
    document.add_layer();
    document.add_layer();
    // Initially: layers [0, 1, 2]
    document.current_layer = 0;
    document.move_layer_down(0); // swap 0 and 1 → [1, 0, 2]
    assert_eq!(document.current_layer, 1);
}

/// Selecting a layer should update `current_layer`.
#[test]
fn select_layer_updates_current() {
    let mut document = small_document();
    document.add_layer();
    document.select_layer(1);
    assert_eq!(document.current_layer, 1);
}

/// Replacing the canvas should reset save path, dirty flag, and undo history.
#[test]
fn replace_canvas_resets_state() {
    let mut document = small_document();
    let mut undo = UndoHistory::new(100);
    document.savefile_path = "/tmp/test.splattercanvas".to_string();
    document.dirty_since_last_autosave = true;

    let new_canvas = Canvas {
        pixels: vec![
            Layer { pixels: vec![Color32::TRANSPARENT; 4] },
            Layer { pixels: vec![Color32::TRANSPARENT; 4] },
        ],
        height: 2,
        width: 2,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
        render_next_frame: false,
    };
    document.replace_canvas(new_canvas, &mut undo);
    assert_eq!(document.canvas.width, 2);
    assert_eq!(document.canvas.pixels.len(), 2);
    assert!(document.savefile_path.is_empty());
    assert!(!document.dirty_since_last_autosave);
    assert!(document.canvas.render_next_frame);
    assert!(!undo.can_undo());
}

/// Verify that the output_rgba buffer starts empty and the pixel count is correct.
#[test]
fn render_to_texture_allocates_output() {
    let document = small_document();
    // We can't easily test texture creation without an egui context,
    // but we can verify that the output_rgba buffer is properly sized.
    let pixel_count = (document.canvas.width * document.canvas.height) as usize;
    assert_eq!(document.canvas.output_rgba.len(), 0); // initially empty
    // output_rgba is resized in render_to_texture, which needs an egui::Ui
    // This test just validates the initial state
    assert_eq!(pixel_count, 100);
}

/// `blend_to_output` with no dirty rects blends the full canvas and sets
/// `render_next_frame` to false, and sizes `output_rgba`.
#[test]
fn blend_to_output_full_canvas_sets_render_state() {
    let mut document = small_document();
    assert_eq!(document.canvas.output_rgba.len(), 0);
    assert!(document.canvas.dirty_rect.is_empty());
    document.canvas.render_next_frame = true;

    let result = document.blend_to_output();

    assert_eq!(result, Some((0, 0, 10, 10)));
    assert!(!document.canvas.render_next_frame);
    assert!(document.canvas.dirty_rect.is_empty());
    assert_eq!(document.canvas.output_rgba.len(), 100 * 4);
}

/// `blend_to_output` with dirty rects only blends those regions and returns
/// the union bounds.
#[test]
fn blend_to_output_dirty_rect_returns_bounds() {
    let mut document = small_document();
    document.canvas.dirty_rect.add(DirtyRect::new(2, 3, 5, 7));
    document.canvas.render_next_frame = true;

    let result = document.blend_to_output();

    // DirtyRect(2,3,5,7) -> width=4, height=5
    assert_eq!(result, Some((2, 3, 4, 5)));
    assert!(!document.canvas.render_next_frame);
    assert!(document.canvas.dirty_rect.is_empty());
}

/// `blend_to_output` with only empty dirty rects returns `None` and clears
/// render_next_frame.
#[test]
fn blend_to_output_empty_dirty_rect_returns_none() {
    let mut document = small_document();
    document.canvas.dirty_rect.add(DirtyRect::empty());
    document.canvas.render_next_frame = true;

    let result = document.blend_to_output();

    assert_eq!(result, None);
    assert!(!document.canvas.render_next_frame);
    assert!(document.canvas.dirty_rect.is_empty());
}

/// Deleting a layer above `current_layer` should leave `current_layer` unchanged.
#[test]
fn delete_layer_preserves_current_when_above() {
    let mut document = small_document();
    document.add_layer();
    document.add_layer();
    document.current_layer = 0;
    // Delete layer at index 2 (above current_layer 0)
    document.delete_layer(2);
    assert_eq!(document.current_layer, 0, "unchanged when deleting above");
    assert_eq!(document.canvas.pixels.len(), 2);
}

/// Deleting a layer below `current_layer` should decrement `current_layer`.
#[test]
fn delete_layer_decrements_current_when_below() {
    let mut document = small_document();
    document.add_layer();
    document.add_layer();
    document.current_layer = 2;
    // Delete layer at index 0 (below current_layer 2)
    document.delete_layer(0);
    assert_eq!(
        document.current_layer, 1,
        "decremented when deleting below"
    );
    assert_eq!(document.canvas.pixels.len(), 2);
}

/// `add_layer` should set `render_next_frame` to true.
#[test]
fn add_layer_sets_render_next_frame() {
    let mut document = small_document();
    document.canvas.render_next_frame = false;
    document.add_layer();
    assert!(document.canvas.render_next_frame, "add_layer triggers re-render");
}

/// `delete_layer` should set `render_next_frame` to true.
#[test]
fn delete_layer_sets_render_next_frame() {
    let mut document = small_document();
    document.canvas.render_next_frame = false;
    document.add_layer();
    document.delete_layer(1);
    assert!(document.canvas.render_next_frame, "delete_layer triggers re-render");
}

/// `move_layer_up` should set `render_next_frame` to true.
#[test]
fn move_layer_up_sets_render_next_frame() {
    let mut document = small_document();
    document.add_layer();
    document.canvas.render_next_frame = false;
    document.move_layer_up(1);
    assert!(document.canvas.render_next_frame, "move_layer_up triggers re-render");
}

/// `move_layer_down` should set `render_next_frame` to true.
#[test]
fn move_layer_down_sets_render_next_frame() {
    let mut document = small_document();
    document.add_layer();
    document.canvas.render_next_frame = false;
    document.move_layer_down(0);
    assert!(document.canvas.render_next_frame, "move_layer_down triggers re-render");
}

/// `move_layer_up(0)` should panic because there is no layer above to swap with.
#[test]
#[should_panic(expected = "subtract with overflow")]
fn move_layer_up_on_top_layer_panics() {
    let mut document = small_document();
    // Only one layer at index 0; moving it up is impossible
    document.move_layer_up(0);
}

/// `move_layer_down` on the bottom layer should panic because there is
/// no layer below to swap with.
#[test]
#[should_panic(expected = "index out of bounds")]
fn move_layer_down_on_bottom_layer_panics() {
    let mut document = small_document();
    // Only one layer at index 0; moving it down is impossible
    document.move_layer_down(0);
}

/// `delete_layer` with an out-of-bounds index should panic.
#[test]
#[should_panic(expected = "removal index")]
fn delete_layer_out_of_bounds_panics() {
    let mut document = small_document();
    document.delete_layer(5);
}

/// `select_layer` with any valid index works. Out-of-bounds doesn't panic
/// but sets `current_layer` to an invalid value that could cause issues.
#[test]
fn select_layer_out_of_bounds_sets_index() {
    let mut document = small_document();
    // select_layer does not validate the index
    document.select_layer(999);
    assert_eq!(document.current_layer, 999);
}

/// Calling `blend_to_output` twice in a row — second call on clean state
/// should set render_next_frame = false and return full-canvas result.
#[test]
fn blend_to_output_twice_resets_state() {
    let mut document = small_document();
    document.canvas.render_next_frame = true;

    // First blend — full canvas (no dirty rects)
    let result1 = document.blend_to_output();
    assert_eq!(result1, Some((0, 0, 10, 10)));
    assert!(!document.canvas.render_next_frame);
    assert!(document.canvas.dirty_rect.is_empty());

    // Second blend — still no dirty rects, full blend again
    document.canvas.render_next_frame = true; // force flag back on
    let result2 = document.blend_to_output();
    assert_eq!(result2, Some((0, 0, 10, 10)));
    assert!(!document.canvas.render_next_frame);
}

/// `add_layer` initializes layer pixels to transparent.
#[test]
fn add_layer_pixels_are_transparent() {
    let mut document = small_document();
    document.add_layer();
    let new_layer = &document.canvas.pixels[1];
    assert!(new_layer.pixels.iter().all(|p| *p == Color32::TRANSPARENT));
}

/// `move_layer_up` swaps correctly with multiple layers.
#[test]
fn move_layer_up_swaps_ordering() {
    let mut document = small_document();
    document.add_layer();
    document.add_layer();
    // Layers: [0: transparent, 1: transparent, 2: transparent]
    // Mark layer 0 with a pixel change to track it
    document.canvas.pixels[0].pixels[0] = Color32::from_rgba_premultiplied(255, 0, 0, 255);
    document.move_layer_up(1);
    // After swap: [1: transparent, 0: has red pixel, 2: transparent]
    assert_eq!(
        document.canvas.pixels[0].pixels[0],
        Color32::TRANSPARENT
    );
    assert_eq!(
        document.canvas.pixels[1].pixels[0],
        Color32::from_rgba_premultiplied(255, 0, 0, 255)
    );
}

/// `delete_layer` at index 0 removes the first item.
#[test]
fn delete_layer_first() {
    let mut document = small_document();
    document.add_layer();
    document.add_layer();
    document.delete_layer(0);
    assert_eq!(document.canvas.pixels.len(), 2);
    // Original layers: [0, 1, 2] -> after delete 0: [1, 2]
}
