# tests/custom_brush

Tests for custom brush line drawing (`tools::custom_brush`). Covers single-tip
placement, drag-line interpolation with spacing, spacing edge-case clamping,
and aspect-ratio-preserving scaling.

## `single_tip_at_center`

Place a single brush tip at canvas center (un-tinted, white tip); verify
2x2 white pixels at expected coordinates.

## `line_interpolates_with_spacing`

Draw a line with spacing=50; verify stamps are placed along the path and center pixel is painted.

## `spacing_zero_clamps_to_minimum_step`

Spacing of 0 should clamp to minimum step of 1 (no panic, still paints).

## `aspect_scaling_rectangular_tip`

Rectangular tip (2x4) paints multiple rows on canvas with aspect ratio preserved.
