# tests::canvas

Tests for `Canvas` construction, default dimensions, and `DirtyRect` bookkeeping.

## Test strategy

- Verify canonical defaults (2000×1500, one transparent layer).
- Exercise `DirtyRect` geometry: empty detection, point extension, union, and default state.

## `default_canvas_size`

`Canvas::default()` creates a 2000×1500 canvas.

## `default_canvas_has_one_transparent_layer`

The default canvas has one layer of `2000 * 1500` transparent pixels.

## `dirty_rect_empty`

`DirtyRect::empty()` reports `is_empty`, zero width, and zero height.

## `dirty_rect_extend_from_empty`

Extending an empty `DirtyRect` with a single point produces a 1×1 rect at that point.

## `dirty_rect_extend_multiple`

Extending a rect with multiple points expands the bounding box to cover all of them.

## `dirty_rect_union`

The union of two overlapping rects produces the minimum bounding box covering both.

## `canvas_dirty_rect_default_none`

A freshly constructed `Canvas` has `dirty_rect: None`.
