# tests::circle_brush

Tests for midpoint-circle span fill (`circle_brush::draw_circle`) and interpolated stamp-line (`circle_brush::draw_circle_line`).

## Test strategy

- Single `draw_circle` at various positions and radii confirms correct pixel coverage and bounds.
- Radius-0 is a no-op; clamping to canvas edge does not panic.
- Multi-layer: circles on different layers modify only their target layer.
- `draw_circle_line` exercises horizontal, vertical, and diagonal interpolation, plus independent stamp deduplication and brush-radius coverage.
- Regression: semi-transparent premultiplied color is stored as-is (no double-darkening).

## `draw_circle_fills_radius_one`

A circle of radius 1 at (5,5) fills the center and four cardinal neighbours.

## `draw_circle_leaves_outside_unchanged`

Pixels far from the circle remain transparent.

## `draw_circle_radius_zero`

A circle of radius 0 is a no-op — the center pixel stays transparent.

## `draw_circle_at_origin`

Drawing a circle at (0,0) with radius 3 does not panic; the origin pixel is colored.

## `draw_circle_multi_layer`

A circle on layer 0 at (2,2) and a circle on layer 1 at (7,7) modify only their respective layers.

## `draw_circle_line_horizontal`

A horizontal stamp line from (1,5) to (8,5) colors both endpoints.

## `draw_circle_line_vertical`

A vertical stamp line from (5,1) to (5,8) colors both endpoints.

## `draw_circle_line_diagonal`

A diagonal stamp line from (1,1) to (8,8) colors both endpoints.

## `draw_circle_line_different_stamps_dont_interfere`

Two stamp lines with different stamp values apply independently without cross-contamination.

## `draw_circle_line_brush_radius`

A brush radius of 3 covers pixels at the center and within the radius; pixels outside remain transparent.

## `draw_circle_line_clamps_to_canvas`

Drawing at (0,0) with a radius-5 brush does not panic; the corner pixel is colored.

## `draw_circle_alpha_overlay_blends`

Alpha-overlay mode for `draw_circle` blends semi-transparent red over opaque white; the result differs from both source and destination.

## `draw_circle_line_alpha_overlay_blends`

Alpha-overlay mode for `draw_circle_line` blends semi-transparent red over opaque white; the result differs from both source and destination.

## `draw_circle_fully_off_screen_returns_empty_undo`

A circle centred at (100, 100) — well outside the 10×10 canvas — returns an `UndoRecord` with an empty runs list.

## Regression: `draw_circle_preserves_premultiplied_semi_transparent`

Semi-transparent premultiplied color (128, 64, 32, 128) is stored verbatim at the center pixel; r=128 is not darkened.

## Regression: `draw_circle_line_preserves_premultiplied_semi_transparent`

Semi-transparent premultiplied color is stored verbatim in stamp-line output.
