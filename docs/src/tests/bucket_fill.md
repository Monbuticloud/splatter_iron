# tests::bucket_fill

Tests for scanline flood-fill (`bucket_fill::draw_bucket_fill`).

## Test strategy

- Fill a pre-drawn red square from its center; verify the region changes and outside pixels are untouched.
- Same-color seed: no-op when target color equals fill color.
- Multi-layer: fill affects only the specified layer.
- Corner seed: fill from `(0, 0)` on a fully saturated canvas.
- Undo record: confirm the returned `UndoRecord` contains run segments.
- Regression: semi-transparent premultiplied fill color must be stored as-is without double-multiplication.

## `canvas_with_red_square` (helper)

Draws a red square at (1,1)–(4,4) on a 10×10 transparent canvas.

## `bucket_fill_fills_contiguous_region`

Filling a red square from its center turns the entire square blue.

## `bucket_fill_does_not_leak`

Pixels outside the red square remain transparent after filling from inside.

## `bucket_fill_same_color_noop`

Filling red with red is a no-op — pixels remain red and outside remains transparent.

## `bucket_fill_multi_layer`

Filling layer 0 at the center of a red square changes only layer 0; layer 1's independent blue square is untouched.

## `bucket_fill_corner_seed`

Filling from `(0, 0)` on a fully red canvas turns the entire canvas blue.

## `bucket_fill_returns_undo`

The returned `UndoRecord` is the `Run` variant with at least one segment.

## Regression: `bucket_fill_preserves_premultiplied_semi_transparent`

A semi-transparent premultiplied fill color (128, 64, 32, 128) is stored verbatim — no double-darkening.
