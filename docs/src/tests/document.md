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
