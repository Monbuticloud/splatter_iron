# tests::square_brush

Tests for rectangular fill (`square_brush::draw_square`) and interpolated stamp-line (`square_brush::draw_square_line`).

## Test strategy

- Single `draw_square` confirms exact pixel coverage (inside and outside the rect).
- Edge cases: clamping to canvas bounds, zero-area rect (no-op), inverted coordinates (empty rect).
- Multi-layer: squares on different layers modify only their target layer.
- `draw_square_line` exercises horizontal, vertical, and diagonal interpolation, plus independent stamp deduplication and brush-radius coverage.
- Regression: semi-transparent premultiplied color is stored as-is (no double-darkening).

## `draw_square_fills_region`

A square from (1,1) to (4,4) fills the interior; top-left and bottom-right are red.

## `draw_square_leaves_outside_unchanged`

Pixels outside the square remain transparent.

## `draw_square_clamps_to_canvas_bounds`

A square extending beyond the 10×10 canvas is clamped; the corner pixel is colored.

## `draw_square_zero_area_is_noop`

A zero-area square at (5,5) does not modify any pixel.

## `draw_square_inverted_coordinates`

A square with start > end produces an empty rect (nothing is colored).

## `draw_square_multi_layer`

A square on layer 0 and a square on layer 1 at different positions modify only their respective layers.

## `draw_square_line_horizontal`

A horizontal stamp line from (1,5) to (8,5) colors both endpoints.

## `draw_square_line_vertical`

A vertical stamp line from (5,1) to (5,8) colors both endpoints.

## `draw_square_line_diagonal`

A diagonal stamp line from (1,1) to (8,8) colors both endpoints.

## `draw_square_line_different_stamps_dont_interfere`

Two stamp lines with different stamp values apply independently.

## `draw_square_line_brush_radius`

A brush radius of 3 covers a 7×7 area around the cursor; pixels outside the radius are untouched.

## `draw_square_line_clamps_to_canvas`

Drawing at (0,0) with a radius-5 brush does not panic; the corner pixel is colored.

## `draw_square_alpha_overlay_blends`

Alpha-overlay mode for `draw_square` blends semi-transparent red over opaque white; the result differs from both source and destination and remains fully opaque.

## `draw_square_line_alpha_overlay_blends`

Alpha-overlay mode for `draw_square_line` blends semi-transparent red over opaque white; the result differs from both source and destination.

## Regression: `draw_square_preserves_premultiplied_semi_transparent`

Semi-transparent premultiplied color (128, 64, 32, 128) is stored verbatim; r=128 is not darkened.

## Regression: `draw_square_line_preserves_premultiplied_semi_transparent`

Semi-transparent premultiplied color is stored verbatim in stamp-line output.
