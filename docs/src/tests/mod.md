# tests

## Module hierarchy

The test suite is organised as one `#[cfg(test)]` module per source module, plus a shared `common` module for test utilities.

| Module               | Tests                                                                                           |
| -------------------- | ----------------------------------------------------------------------------------------------- |
| `common`             | Shared helpers: `small_canvas()`, premultiplied `red()` and `blue()` shorthands                 |
| `pixel`              | Premultiplied-alpha math, SIMD blend, `blend_layers`, `blend_region`                            |
| `undo`               | `compress_run`, `undo_apply`, `redo_apply` — stroke-pixel round-trips                           |
| `undo_history`       | `UndoHistory` stack management, visited-stamp dedup, multi-step undo/redo                       |
| `canvas`             | `Canvas` defaults, `DirtyRect` bookkeeping                                                      |
| `document`           | Layer add/delete/reorder/select, `blend_to_output`, `replace_canvas`                            |
| `files`              | Serialization round-trips (zstd/JSON), PNG/JPEG export/import, error handling                   |
| `file_io`            | Async file-dialog mpsc plumbing, save/load orchestration                                        |
| `bucket_fill`        | Scanline flood-fill — contiguous regions, leaks, multi-layer, premultiplied-alpha preservation  |
| `circle_brush`       | Midpoint-circle span fill, stamp-line interpolation, edge clamping                              |
| `square_brush`       | Rectangular fill, stamp-line interpolation, coordinate clamping, zero-area edge cases           |
| `stamp_brush`        | Single-stamp placement, scaling, tinting, alpha overlay, drag-line interpolation, edge clamping |
| `tool_configuration` | `ToolConfiguration` default values and optional-field initial state                             |

## Test strategy

- Every public function in the crate has at least one test entry point.
- Regression tests for double-premultiply bugs (the most common correctness defect in the codebase) are present in `pixel`, `bucket_fill`, `circle_brush`, and `square_brush`.
- Edge cases: zero-area brushes, bounding-box clamping, inverted coordinates, empty layer lists, wrapping stamp counters, off-screen stamps, `MAX_STROKE_STACK` eviction.
- Round-trip tests verify serialisation symmetry for zstd/JSON, PNG, WebP, GIF, and TIFF image formats.
- Asynchronous file-IO tests exercise the mpsc channel plumbing without spawning real UI dialogs.

## `app`

Application-level constants, UIState defaults, export format metadata, and PendingStamp construction.

## `brush_library`

BrushLibrary add/remove/select/persistence round-trip tests.

## `brush_parsers`

GBR and ABR brush format file parser tests with synthetic byte buffers.

## `custom_brush`

Custom brush tip placement, line interpolation with spacing, and aspect-ratio scaling.

## `stamp_library`

StampLibrary add/remove/select/persistence round-trip tests.
