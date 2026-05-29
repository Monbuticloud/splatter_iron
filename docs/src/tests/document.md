# tests::document

Tests for `Document` — layer management, blend-to-output compositing, and canvas replacement.

## Test strategy

- Layer operations: add, delete, reorder (`move_layer_up`/`move_layer_down`), select.
- State resets: `replace_canvas` clears save path, dirty flag, and undo history.
- Blend pipeline: `blend_to_output` with full-canvas, dirty-rect, and empty-dirty-rect inputs.

## `new_document_has_one_layer`

A fresh `Document` wrapping a 10×10 canvas has one layer, current_layer=0, no save path, and dirty_since_last_autosave=false.

## `add_layer_increases_count`

Adding a layer increments the layer count and sets `render_next_frame = true`.

## `add_layer_has_correct_size`

A newly added layer's pixel buffer matches `width * height`.

## `delete_layer_removes_correct_index`

Deleting a specific index removes the correct layer from the stack.

## `delete_last_layer_removes_it`

Deleting the only layer leaves the document with zero layers (the UI layer protects against this).

## `delete_layer_adjusts_current_layer_down`

Deleting the current layer saturates `current_layer` to the new max index.

## `move_layer_up_swaps`

Moving a layer up swaps it with the layer above and updates `current_layer`.

## `move_layer_down_swaps`

Moving a layer down swaps it with the layer below and updates `current_layer`.

## `select_layer_updates_current`

`select_layer` sets `current_layer` to the requested index.

## `replace_canvas_resets_state`

Replacing the canvas clears the save path, dirty flag, and undo history, and requests a re-render.

## `render_to_texture_allocates_output`

The `output_rgba` buffer is initially empty; `pixel_count = width * height` is correct.

## `blend_to_output_full_canvas_sets_render_state`

Blending the full canvas returns `Some((0, 0, width, height))`, clears `render_next_frame` and `dirty_rect`, and sizes `output_rgba`.

## `blend_to_output_dirty_rect_returns_bounds`

With a dirty rect set, `blend_to_output` returns the dirty-rect bounds (translated to width/height) and clears render state.

## `blend_to_output_empty_dirty_rect_returns_none`

With an empty `DirtyRect`, `blend_to_output` returns `None` and clears render state.

## `delete_layer_preserves_current_when_above`

Deleting a layer at index 2 when `current_layer = 0` leaves `current_layer` unchanged at 0.

## `delete_layer_decrements_current_when_below`

Deleting a layer at index 0 when `current_layer = 2` decrements `current_layer` to 1.

## `add_layer_sets_render_next_frame`

`add_layer` sets `render_next_frame` to `true` after adding a transparent layer.

## `delete_layer_sets_render_next_frame`

`delete_layer` sets `render_next_frame` to `true` after removing a layer.

## `move_layer_up_sets_render_next_frame`

`move_layer_up` sets `render_next_frame` to `true` after swapping layers.

## `move_layer_down_sets_render_next_frame`

`move_layer_down` sets `render_next_frame` to `true` after swapping layers.

## `blend_to_output_empty_dirty_rect_triggers_full_blend`

With an empty DirtyRect (DirtyRectList), blend_to_output performs a full-canvas blend and returns Some((0,0,10,10)) because the empty-list triggers a full blend. Previously documented as returning None.

## `move_layer_up_on_top_layer_panics`

move_layer_up on the top layer (index 0) panics as documented.

## `move_layer_down_on_bottom_layer_panics`

move_layer_down on the bottom layer panics as documented.

## `delete_layer_out_of_bounds_panics`

Deleting a layer with out-of-bounds index panics.
