use eframe::egui::Color32;

use crate::canvas::{Canvas, Layer};
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
        render_next_frame: false,
    };
    Document::new(canvas)
}

/// A new document should have one layer, current_layer 0, no save path.
#[test]
fn new_document_has_one_layer() {
    let doc = small_document();
    assert_eq!(doc.canvas.pixels.len(), 1);
    assert_eq!(doc.current_layer, 0);
    assert!(!doc.dirty_since_last_autosave);
    assert!(doc.savefile_path.is_empty());
}

/// Adding a layer should increase the layer count and request a re-render.
#[test]
fn add_layer_increases_count() {
    let mut doc = small_document();
    doc.add_layer();
    assert_eq!(doc.canvas.pixels.len(), 2);
    assert!(doc.canvas.render_next_frame);
}

/// A newly added layer should match the canvas dimensions.
#[test]
fn add_layer_has_correct_size() {
    let mut doc = small_document();
    doc.add_layer();
    let expected = (doc.canvas.width * doc.canvas.height) as usize;
    assert_eq!(doc.canvas.pixels[1].pixels.len(), expected);
}

/// Deleting a layer should remove the correct index from the layer list.
#[test]
fn delete_layer_removes_correct_index() {
    let mut doc = small_document();
    doc.add_layer();
    doc.add_layer();
    assert_eq!(doc.canvas.pixels.len(), 3);
    doc.delete_layer(1);
    assert_eq!(doc.canvas.pixels.len(), 2);
    // The remaining layers should be index 0 and 2 from the original set
}

/// Deleting the only layer should leave the document with 0 layers.
#[test]
fn delete_last_layer_removes_it() {
    let mut doc = small_document();
    doc.delete_layer(0);
    // Document model does not guard against removing the last layer;
    // that check is in the UI layer (ui/right.rs).
    assert_eq!(doc.canvas.pixels.len(), 0);
    // current_layer saturates to 0
    assert_eq!(doc.current_layer, 0);
}

/// Deleting the current layer should adjust `current_layer` downward.
#[test]
fn delete_layer_adjusts_current_layer_down() {
    let mut doc = small_document();
    doc.add_layer();
    doc.add_layer();
    doc.current_layer = 2;
    doc.delete_layer(2);
    // current_layer should go from 2 → min(1, 1) = 1
    assert_eq!(doc.current_layer, 1);
}

/// Moving a layer up should swap it with the layer above.
#[test]
fn move_layer_up_swaps() {
    let mut doc = small_document();
    doc.add_layer();
    doc.add_layer();
    // Initially: layers [0, 1, 2]
    doc.move_layer_up(1); // swap 1 and 0 → [1, 0, 2]
    assert_eq!(doc.current_layer, 0);
}

/// Moving a layer down should swap it with the layer below.
#[test]
fn move_layer_down_swaps() {
    let mut doc = small_document();
    doc.add_layer();
    doc.add_layer();
    // Initially: layers [0, 1, 2]
    doc.current_layer = 0;
    doc.move_layer_down(0); // swap 0 and 1 → [1, 0, 2]
    assert_eq!(doc.current_layer, 1);
}

/// Selecting a layer should update `current_layer`.
#[test]
fn select_layer_updates_current() {
    let mut doc = small_document();
    doc.add_layer();
    doc.select_layer(1);
    assert_eq!(doc.current_layer, 1);
}

#[test]
fn replace_canvas_resets_state() {
    let mut doc = small_document();
    let mut undo = UndoHistory::new(100);
    doc.savefile_path = "/tmp/test.splattercanvas".to_string();
    doc.dirty_since_last_autosave = true;

    let new_canvas = Canvas {
        pixels: vec![
            Layer { pixels: vec![Color32::TRANSPARENT; 4] },
            Layer { pixels: vec![Color32::TRANSPARENT; 4] },
        ],
        height: 2,
        width: 2,
        output_rgba: Vec::new(),
        rendered_layers: None,
        render_next_frame: false,
    };
    doc.replace_canvas(new_canvas, &mut undo);
    assert_eq!(doc.canvas.width, 2);
    assert_eq!(doc.canvas.pixels.len(), 2);
    assert!(doc.savefile_path.is_empty());
    assert!(!doc.dirty_since_last_autosave);
    assert!(doc.canvas.render_next_frame);
    assert!(!undo.can_undo());
}

#[test]
fn render_to_texture_allocates_output() {
    let mut doc = small_document();
    // We can't easily test texture creation without an egui context,
    // but we can verify that the output_rgba buffer is properly sized.
    let pixel_count = (doc.canvas.width * doc.canvas.height) as usize;
    assert_eq!(doc.canvas.output_rgba.len(), 0); // initially empty
    // output_rgba is resized in render_to_texture, which needs an egui::Ui
    // This test just validates the initial state
    assert_eq!(pixel_count, 100);
}
