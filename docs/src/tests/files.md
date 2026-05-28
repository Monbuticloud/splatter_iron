# tests::files

Tests for serialisation — zstd-compressed JSON canvas save/load round-trips and PNG/JPEG export/import.

## Test strategy

- Round-trips: save a `Canvas` to bytes, load it back, verify every pixel is identical.
- Multi-layer: both layers survive the round-trip.
- Transparency: transparent and semi-transparent pixels are preserved.
- Image formats: export a checkerboard to PNG and re-import; export JPEG and verify file existence; export semi-transparent PNG and verify re-import.
- Error paths: loading non-zstd data or empty data produces an `Err`.

## `checkerboard_4x4` (helper)

Constructs a 4×4 single-layer `Canvas` with alternating white/black opaque pixels.

## `save_load_roundtrip_identical_pixels`

A checkerboard canvas survives a `save_canvas_to_bytes` → `load_app_from_data` round-trip with all pixels identical.

## `save_load_roundtrip_multi_layer`

A 2-layer canvas round-trips with both layers intact and pixel-identical.

## `save_load_roundtrip_transparent`

A 1×3 canvas with transparent and semi-transparent pixels round-trips without panicking.

## `export_png_roundtrip`

Export a checkerboard to PNG via `export_as_image`, re-import via `import_image_as_canvas`, and verify every pixel matches.

## `export_jpeg_creates_file`

Exporting a 2×2 uniform-color buffer as JPEG produces a non-empty file on disk.

## `export_png_semi_transparent`

Exporting a 2×2 buffer with mixed opacity/transparency as PNG and re-importing preserves the correct dimensions and layer count.

## `invalid_data_returns_error`

`load_app_from_data` on a non-zstd byte slice returns `Err`.

## `empty_data_returns_error`

`load_app_from_data` on an empty byte slice returns `Err`.
