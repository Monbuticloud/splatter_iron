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

## `canvas_new_sets_dimensions`

`Canvas::new(42, 24)` creates a canvas with `width=42`, `height=24`, one transparent layer, and the correct pixel count.

## `default_render_next_frame_is_true`

The default canvas sets `render_next_frame` to `true` (initial full re-render needed).

## `canvas_new_render_next_frame_is_true`

`Canvas::new(10, 10)` also sets `render_next_frame` to `true`.

## `canvas_dirty_rect_default_empty`

A freshly constructed Canvas has dirty_rect.is_empty() (DirtyRectList starts empty). Renamed from canvas_dirty_rect_default_none to reflect the DirtyRectList type.

## `canvas_serde_roundtrip`

Canvas serializes and deserializes with identical pixels via serde_json.

## `canvas_serde_multi_layer`

A multi-layer canvas round-trips correctly through serde.

## `dirty_rect_inverted_is_empty`

A DirtyRect with inverted bounds (min > max) reports is_empty.

## `dirty_rect_extend_fixes_inverted`

Extending an inverted DirtyRect corrects the bounds.

## `dirty_rect_union_with_empty`

Unioning a valid rect with an empty rect produces the valid rect.

## `dirty_rect_single_pixel`

A DirtyRect created via new with identical min/max is a 1x1 rect.

## `dirty_rect_default_covers_origin`

DirtyRect::empty() has inverted bounds covering the origin sentinel.

## `dirty_rect_list_empty`

DirtyRectList::new creates an empty list; is_empty returns true.

## `dirty_rect_list_add_single`

Adding one DirtyRect to DirtyRectList stores it.

## `dirty_rect_list_non_overlapping_separate`

Non-overlapping rects remain separate in DirtyRectList.
