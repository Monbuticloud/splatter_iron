# tests/custom_brush

Tests for custom brush line drawing (`tools::custom_brush`). Covers single-tip
placement, drag-line interpolation with spacing, spacing edge-case clamping,
and aspect-ratio-preserving scaling.

## `single_tip_at_center`

Place a single brush tip at canvas center (un-tinted, white tip); verify
2x2 white pixels at expected coordinates.

## `line_interpolates_with_spacing`

Draw a line with spacing=50; verify stamps are placed along the path and center pixel is painted.
