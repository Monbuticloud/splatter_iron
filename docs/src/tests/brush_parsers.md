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

## `parse_gbr_v1`

Parse a GBR v1 file (no spacing) and verify default spacing of 25.

## `parse_gbr_invalid_magic`

Invalid GBR magic bytes should return an error.

## `parse_gbr_truncated`

Truncated GBR file should return an error.

## `parse_gbr_zero_dimensions`

GBR with zero width or height should return an error.

## `parse_gbr_unsupported_bpp`

GBR with unsupported bytes-per-pixel should return an error.

## `parse_abr_invalid_magic`

Invalid ABR magic bytes should return an error.

## `parse_abr_truncated`

Truncated ABR file should return an error.
