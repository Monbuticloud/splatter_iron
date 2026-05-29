# tests::stamp_brush

Tests for stamp brush (`stamp_brush::draw_stamp_line`).

## Test strategy

- Single stamp placement verifies correct pixel mapping from source to output.
- Minimum radius (0) produces a 1×1 output.
- Tinted mode multiplies stamp pixels by the tint colour.
- Alpha-overlay blends stamp over existing background.
- Edge clamping at canvas corners and oversized stamps do not panic.
- Drag-line interpolation places stamps along the path.
- Rectangular (non-square) stamps preserve aspect ratio.
- Fully off-screen stamp produces no visible change.

## `solid_stamp` (helper)

Builds a 2×2 stamp: top-left red, top-right green, bottom-left blue, bottom-right white.

## `single_stamp_at_center`

A radius-2 stamp centred at (5,5) places the 2×2 source image at output (4–5, 4–5).

## `stamp_minimum_radius`

Radius 0 produces a 1×1 output mapping to source (0,0).

## `tinted_stamp_applies_color`

Tinted mode multiplies stamp pixels by the tint colour: red \* (128,128,128,255) → (128,0,0,255).

## `alpha_overlay_blends_stamp`

Alpha-overlay mode blends semi-transparent red stamp over an existing blue background; result differs from both source and destination.

## `stamp_clamps_to_canvas_edge`

A stamp centred at the canvas origin with radius 4 clips correctly; the visible pixel maps to the correct source coordinate via nearest-neighbour sampling.

## `stamp_line_interpolates`

A drag line from (2,2) to (7,7) with radius 4 places multiple stamps along the diagonal; the endpoint is covered.

## `oversized_stamp_clamps`

A stamp with radius 100 (much larger than the 10×10 canvas) covers all four corners without panicking.

## `stamp_produces_valid_undo_record`

The returned `UndoRecord` targets layer 0 and contains at least one run segment.

## `stamp_does_not_affect_outside`

Pixels far from the stamp bounds remain transparent.

## `stamp_rectangular_aspect`

A 4×1 non-square stamp preserves its aspect ratio when placed at radius 4; left and right output pixels map to the correct source coordinates.

## `stamp_fully_off_screen_noop`

A stamp centred at (100, 100) — far outside the canvas — leaves all pixels transparent and produces no visible effect.
