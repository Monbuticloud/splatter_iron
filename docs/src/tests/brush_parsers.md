# tests/brush_parsers

Tests for GBR and ABR brush format parsing. Constructs synthetic byte buffers
and verifies correct pixel decoding, spacing, error handling, and brush
rasterisation.

## `parse_gbr_v2_rgba_basic`

Parse a valid GBR v2 RGBA brush and verify dimensions + pixel count.

## `parse_gbr_v2_rgba_pixels`

Verify RGBA pixel values, premultiplied format, and per-pixel channel constraints.

## `parse_gbr_v2_grayscale`

Parse a grayscale GBR brush and verify correct alpha-derived premultiplied values.
