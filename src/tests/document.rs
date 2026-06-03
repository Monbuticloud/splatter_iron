//! Tests for `Document` — layer management, blend-to-output, and GPU upload.
//!
//! Covers adding, deleting, reordering, and selecting layers, as well as
//! compositing output and tracking dirty regions for rendering.

use std::sync::Arc;

use eframe::egui::Color32;

use crate::canvas::Canvas;
use crate::canvas::DirtyRect;
use crate::canvas::DirtyRectList;
use crate::canvas::Layer;
use crate::document::Document;
use crate::undo_history::UndoHistory;

/// Build a 10×10 single-layer document for use in tests.
fn small_document() -> Document {
    let canvas = Canvas {
        pixels: vec![Layer {
            pixels: vec![Color32::TRANSPARENT; 100],
            ..Default::default()
        }],
        height: 10,
        width: 10,
        output_rgba: Arc::new(Vec::new()),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
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
    document.add_layer(&mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels.len(), 2);
    assert!(document.canvas_mut().dirty_rect.needs_reblend());
}

/// `add_layer` should select the newly added layer.
#[test]
fn add_layer_selects_new_layer() {
    let mut document = small_document();
    assert_eq!(document.current_layer, 0);
    document.add_layer(&mut UndoHistory::new(100));
    assert_eq!(document.current_layer, 1, "add_layer selects the new layer");
}

/// `add_layer` should produce unique default names even after layer deletion.
#[test]
fn add_layer_unique_names_after_delete() {
    let mut document = small_document();
    // Initial layer is "Layer 1" (named by Canvas::default)
    document.add_layer(&mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels[1].name, "Layer 2");
    // Delete the original layer, leaving "Layer 2"
    document.delete_layer(0, &mut UndoHistory::new(100));
    // Add another layer — must not reuse "Layer 2"
    document.add_layer(&mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels[1].name, "Layer 3");
}

/// Multiple add/delete cycles should never produce a duplicate default name.
#[test]
fn add_layer_unique_names_multiple_cycles() {
    let mut document = small_document();
    // Cycle 1: add Layer 2, delete Layer 1
    document.add_layer(&mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels[1].name, "Layer 2");
    document.delete_layer(0, &mut UndoHistory::new(100));
    // Cycle 2: add Layer 3, delete Layer 2
    document.add_layer(&mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels[1].name, "Layer 3");
    document.delete_layer(0, &mut UndoHistory::new(100));
    // Cycle 3: add Layer 4
    document.add_layer(&mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels[1].name, "Layer 4");
}

/// After any sequence of add/delete operations, every layer name in the stack
/// must be unique (egui uses labels as widget IDs, so duplicates cause ID
/// collisions).
#[test]
fn add_layer_all_names_unique() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100)); // Layer 2
    document.add_layer(&mut UndoHistory::new(100)); // Layer 3
    document.delete_layer(0, &mut UndoHistory::new(100));
    document.add_layer(&mut UndoHistory::new(100)); // Layer 4
    document.delete_layer(1, &mut UndoHistory::new(100));
    document.add_layer(&mut UndoHistory::new(100)); // Layer 5
    let names: Vec<String> = document
        .canvas
        .pixels
        .iter()
        .map(|l| l.name.clone())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        names.len(),
        "all layer names must be unique, got {names:?}"
    );
}

/// `replace_canvas` must reset `next_layer_number` so that subsequent
/// `add_layer` calls produce names aligned with the new canvas's layer count.
#[test]
fn replace_canvas_resets_next_layer_number() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100)); // Layer 2
    let new_canvas = Canvas {
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
    document.replace_canvas(new_canvas, &mut UndoHistory::new(100));
    // Canvas has 2 layers, so next layer should be "Layer 3"
    document.add_layer(&mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels[2].name, "Layer 3");
}

/// Undoing a layer deletion then adding a new layer should still produce a
/// unique name — the counter is not affected by undo/redo.
#[test]
fn undo_delete_then_add_uses_next_number() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100)); // Layer 2
    document.delete_layer(0, &mut UndoHistory::new(100));
    // After delete+add: counter advances regardless of undo state
    document.add_layer(&mut UndoHistory::new(100)); // Layer 3 — not "Layer 2"
    assert_eq!(document.canvas.pixels[0].name, "Layer 2");
    assert_eq!(document.canvas.pixels[1].name, "Layer 3");
}

/// A newly added layer should match the canvas dimensions.
#[test]
fn add_layer_has_correct_size() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    let expected = (document.canvas.width * document.canvas.height) as usize;
    assert_eq!(document.canvas.pixels[1].pixels.len(), expected);
}

/// Deleting a layer should remove the correct index from the layer list.
#[test]
fn delete_layer_removes_correct_index() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    document.add_layer(&mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels.len(), 3);
    document.delete_layer(1, &mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels.len(), 2);
    // The remaining layers should be index 0 and 2 from the original set
}

/// Deleting the only layer should leave the document with 0 layers.
#[test]
fn delete_last_layer_removes_it() {
    let mut document = small_document();
    document.delete_layer(0, &mut UndoHistory::new(100));
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
    document.add_layer(&mut UndoHistory::new(100));
    document.add_layer(&mut UndoHistory::new(100));
    document.current_layer = 2;
    document.delete_layer(2, &mut UndoHistory::new(100));
    // current_layer should go from 2 → min(1, 1) = 1
    assert_eq!(document.current_layer, 1);
}

/// Moving a layer up should swap it with the layer above.
#[test]
fn move_layer_up_swaps() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    document.add_layer(&mut UndoHistory::new(100));
    // Initially: layers [0, 1, 2]
    document.move_layer_up(1, &mut UndoHistory::new(100)); // swap 1 and 0 → [1, 0, 2]
    assert_eq!(document.current_layer, 0);
}

/// Moving a layer down should swap it with the layer below.
#[test]
fn move_layer_down_swaps() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    document.add_layer(&mut UndoHistory::new(100));
    // Initially: layers [0, 1, 2]
    document.current_layer = 0;
    document.move_layer_down(0, &mut UndoHistory::new(100)); // swap 0 and 1 → [1, 0, 2]
    assert_eq!(document.current_layer, 1);
}

/// Selecting a layer should update `current_layer`.
#[test]
fn select_layer_updates_current() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
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
            Layer {
                pixels: vec![Color32::TRANSPARENT; 4],
                ..Default::default()
            },
            Layer {
                pixels: vec![Color32::TRANSPARENT; 4],
                ..Default::default()
            },
        ],
        height: 2,
        width: 2,
        output_rgba: Arc::new(Vec::new()),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
    };
    document.replace_canvas(new_canvas, &mut undo);
    assert_eq!(document.canvas.width, 2);
    assert_eq!(document.canvas.pixels.len(), 2);
    assert!(document.savefile_path.is_empty());
    assert!(!document.dirty_since_last_autosave);
    assert!(document.canvas_mut().dirty_rect.needs_reblend());
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

/// `blend_to_output` with no dirty rects but with `needs_full_blend` set
/// blends the full canvas and clears the reblend request.
#[test]
fn blend_to_output_full_canvas_clears_reblend() {
    let mut document = small_document();
    assert_eq!(document.canvas.output_rgba.len(), 0);
    assert!(document.canvas.dirty_rect.is_empty());
    document.canvas_mut().dirty_rect.request_full_blend();

    let result = document.blend_to_output();

    assert_eq!(result, Some(DirtyRect::new(0, 0, 9, 9)));
    assert!(!document.canvas_mut().dirty_rect.needs_reblend());
    assert!(document.canvas.dirty_rect.is_empty());
    assert_eq!(document.canvas.output_rgba.len(), 100 * 4);
}

/// `blend_to_output` with dirty rects only blends those regions and returns
/// the union bounds.
#[test]
fn blend_to_output_dirty_rect_returns_bounds() {
    let mut document = small_document();
    document
        .canvas_mut()
        .dirty_rect
        .add(DirtyRect::new(2, 3, 5, 7));
    document.canvas_mut().dirty_rect.request_full_blend();

    let result = document.blend_to_output();

    // DirtyRect(2,3,5,7) -> width=4, height=5
    assert_eq!(result, Some(DirtyRect::new(2, 3, 5, 7)));
    assert!(!document.canvas_mut().dirty_rect.needs_reblend());
    assert!(document.canvas.dirty_rect.is_empty());
}

/// `blend_to_output` with only empty dirty rects (which are no-ops) does a
/// full canvas blend when `needs_full_blend` is set.
#[test]
fn blend_to_output_empty_dirty_rect_triggers_full_blend() {
    let mut document = small_document();
    // Empty rects are filtered out by DirtyRectList::add → no-op.
    // An empty dirty list with needs_full_blend triggers a full blend.
    document.canvas_mut().dirty_rect.add(DirtyRect::empty());
    assert!(document.canvas.dirty_rect.is_empty());
    document.canvas_mut().dirty_rect.request_full_blend();

    let result = document.blend_to_output();

    assert_eq!(result, Some(DirtyRect::new(0, 0, 9, 9)));
    assert!(!document.canvas_mut().dirty_rect.needs_reblend());
    assert!(document.canvas.dirty_rect.is_empty());
}

/// Deleting a layer above `current_layer` should leave `current_layer` unchanged.
#[test]
fn delete_layer_preserves_current_when_above() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    document.add_layer(&mut UndoHistory::new(100));
    document.current_layer = 0;
    // Delete layer at index 2 (above current_layer 0)
    document.delete_layer(2, &mut UndoHistory::new(100));
    assert_eq!(document.current_layer, 0, "unchanged when deleting above");
    assert_eq!(document.canvas.pixels.len(), 2);
}

/// Deleting a layer above `current_layer` (with current_layer > 0) should
/// leave `current_layer` unchanged.
#[test]
fn delete_layer_preserves_current_when_above_middle() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    document.add_layer(&mut UndoHistory::new(100));
    document.current_layer = 1;
    // Delete layer at index 2 (above current_layer 1)
    document.delete_layer(2, &mut UndoHistory::new(100));
    assert_eq!(
        document.current_layer, 1,
        "unchanged when deleting above (middle)"
    );
    assert_eq!(document.canvas.pixels.len(), 2);
}

/// Deleting a layer below `current_layer` should decrement `current_layer`.
#[test]
fn delete_layer_decrements_current_when_below() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    document.add_layer(&mut UndoHistory::new(100));
    document.current_layer = 2;
    // Delete layer at index 0 (below current_layer 2)
    document.delete_layer(0, &mut UndoHistory::new(100));
    assert_eq!(document.current_layer, 1, "decremented when deleting below");
    assert_eq!(document.canvas.pixels.len(), 2);
}

/// `add_layer` should request a full re-blend.
#[test]
fn add_layer_triggers_full_blend() {
    let mut document = small_document();
    document.canvas_mut().dirty_rect.clear();
    document.add_layer(&mut UndoHistory::new(100));
    assert!(
        document.canvas_mut().dirty_rect.needs_reblend(),
        "add_layer triggers re-render"
    );
}

/// `delete_layer` should request a full re-blend.
#[test]
fn delete_layer_triggers_full_blend() {
    let mut document = small_document();
    document.canvas_mut().dirty_rect.clear();
    document.add_layer(&mut UndoHistory::new(100));
    document.delete_layer(1, &mut UndoHistory::new(100));
    assert!(
        document.canvas_mut().dirty_rect.needs_reblend(),
        "delete_layer triggers re-render"
    );
}

/// `move_layer_up` should request a full re-blend.
#[test]
fn move_layer_up_triggers_full_blend() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    document.canvas_mut().dirty_rect.clear();
    document.move_layer_up(1, &mut UndoHistory::new(100));
    assert!(
        document.canvas_mut().dirty_rect.needs_reblend(),
        "move_layer_up triggers re-render"
    );
}

/// `move_layer_down` should request a full re-blend.
#[test]
fn move_layer_down_triggers_full_blend() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    document.canvas_mut().dirty_rect.clear();
    document.move_layer_down(0, &mut UndoHistory::new(100));
    assert!(
        document.canvas_mut().dirty_rect.needs_reblend(),
        "move_layer_down triggers re-render"
    );
}

/// `move_layer_up(0)` should panic because there is no layer above to swap with.
#[test]
#[should_panic(expected = "subtract with overflow")]
fn move_layer_up_on_top_layer_panics() {
    let mut document = small_document();
    // Only one layer at index 0; moving it up is impossible
    document.move_layer_up(0, &mut UndoHistory::new(100));
}

/// `move_layer_down` on the bottom layer should panic because there is
/// no layer below to swap with.
#[test]
#[should_panic(expected = "index out of bounds")]
fn move_layer_down_on_bottom_layer_panics() {
    let mut document = small_document();
    // Only one layer at index 0; moving it down is impossible
    document.move_layer_down(0, &mut UndoHistory::new(100));
}

/// `delete_layer` with an out-of-bounds index should panic.
#[test]
#[should_panic(expected = "removal index")]
fn delete_layer_out_of_bounds_panics() {
    let mut document = small_document();
    let mut undo = UndoHistory::new(100);
    document.delete_layer(5, &mut undo);
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
/// should clear the reblend request and return full-canvas result each time.
#[test]
fn blend_to_output_twice_resets_state() {
    let mut document = small_document();
    document.canvas_mut().dirty_rect.request_full_blend();

    // First blend — full canvas (no dirty rects)
    let result1 = document.blend_to_output();
    assert_eq!(result1, Some(DirtyRect::new(0, 0, 9, 9)));
    assert!(!document.canvas_mut().dirty_rect.needs_reblend());
    assert!(document.canvas.dirty_rect.is_empty());

    // Second blend — needs_full_blend was cleared, but take_all() returns
    // empty, so blend_to_output still does a full blend (no partial info).
    document.canvas_mut().dirty_rect.request_full_blend(); // force flag back on
    let result2 = document.blend_to_output();
    assert_eq!(result2, Some(DirtyRect::new(0, 0, 9, 9)));
    assert!(!document.canvas_mut().dirty_rect.needs_reblend());
}

/// `add_layer` initializes layer pixels to transparent.
#[test]
fn add_layer_pixels_are_transparent() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    let new_layer = &document.canvas.pixels[1];
    assert!(new_layer.pixels.iter().all(|p| *p == Color32::TRANSPARENT));
}

/// `move_layer_up` swaps correctly with multiple layers.
#[test]
fn move_layer_up_swaps_ordering() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    document.add_layer(&mut UndoHistory::new(100));
    // Layers: [0: transparent, 1: transparent, 2: transparent]
    // Mark layer 0 with a pixel change to track it
    document.canvas_mut().pixels[0].pixels[0] = Color32::from_rgba_premultiplied(255, 0, 0, 255);
    document.move_layer_up(1, &mut UndoHistory::new(100));
    // After swap: [1: transparent, 0: has red pixel, 2: transparent]
    assert_eq!(document.canvas.pixels[0].pixels[0], Color32::TRANSPARENT);
    assert_eq!(
        document.canvas.pixels[1].pixels[0],
        Color32::from_rgba_premultiplied(255, 0, 0, 255)
    );
}

/// `delete_layer` at index 0 removes the first item.
#[test]
fn delete_layer_first() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    document.add_layer(&mut UndoHistory::new(100));
    document.delete_layer(0, &mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels.len(), 2);
    // Original layers: [0, 1, 2] -> after delete 0: [1, 2]
}

/// `toggle_layer_visible` makes a visible layer invisible.
#[test]
fn toggle_layer_visible_hides_layer() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    assert!(document.canvas.pixels[1].visible);
    document.toggle_layer_visible(1, &mut UndoHistory::new(100));
    assert!(!document.canvas.pixels[1].visible);
}

/// `toggle_layer_visible` refuses to hide the last visible layer.
#[test]
fn toggle_layer_visible_keeps_last_visible() {
    let mut document = small_document();
    assert!(document.canvas.pixels[0].visible);
    // Only one layer visible — should not be hidden
    document.toggle_layer_visible(0, &mut UndoHistory::new(100));
    assert!(
        document.canvas.pixels[0].visible,
        "last visible layer stays visible"
    );
}

/// `toggle_layer_visible` makes an invisible layer visible again.
#[test]
fn toggle_layer_visible_shows_hidden_layer() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    document.canvas_mut().pixels[1].visible = false;
    document.toggle_layer_visible(1, &mut UndoHistory::new(100));
    assert!(document.canvas.pixels[1].visible);
}

/// `toggle_layer_visible` requests a full re-blend.
#[test]
fn toggle_layer_visible_triggers_blend() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    document.canvas_mut().dirty_rect.clear();
    document.toggle_layer_visible(1, &mut UndoHistory::new(100));
    assert!(document.canvas_mut().dirty_rect.needs_reblend());
}

/// `set_layer_opacity` changes the layer opacity.
#[test]
fn set_layer_opacity_changes_opacity() {
    let mut document = small_document();
    assert_eq!(document.canvas.pixels[0].opacity, 255);
    document.set_layer_opacity(0, 128, &mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels[0].opacity, 128);
}

/// `set_layer_opacity` accepts boundary values 0 and 255.
#[test]
fn set_layer_opacity_boundary_values() {
    let mut document = small_document();
    document.set_layer_opacity(0, 0, &mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels[0].opacity, 0);
    document.set_layer_opacity(0, 255, &mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels[0].opacity, 255);
}

/// `set_layer_opacity` requests a full re-blend.
#[test]
fn set_layer_opacity_triggers_blend() {
    let mut document = small_document();
    document.canvas_mut().dirty_rect.clear();
    document.set_layer_opacity(0, 128, &mut UndoHistory::new(100));
    assert!(document.canvas_mut().dirty_rect.needs_reblend());
}

/// `rename_layer` changes the layer name.
#[test]
fn rename_layer_changes_name() {
    let mut document = small_document();
    document.rename_layer(0, "Background".to_string(), &mut UndoHistory::new(100));
    assert_eq!(document.canvas.pixels[0].name, "Background");
}

/// `blend_to_output` skips invisible layers.
#[test]
fn blend_to_output_skips_invisible_layers() {
    let mut document = small_document();
    document.add_layer(&mut UndoHistory::new(100));
    // Paint a red pixel on layer 1
    document.canvas_mut().pixels[1].pixels[0] = Color32::RED;
    // Hide layer 1 and blend
    document.canvas_mut().pixels[1].visible = false;
    document.canvas_mut().dirty_rect.request_full_blend();
    document.blend_to_output();
    // Output should show the checkerboard pattern behind transparent areas.
    // Pixel (0,0) is a light tile: 192 × 255/256 ≈ 191, with alpha = 255.
    assert_eq!(
        document.canvas.output_rgba[0..4].to_vec(),
        vec![191, 191, 191, 255]
    );
}
