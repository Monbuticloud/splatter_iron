# SplatterIron — Agent Guide

## Build & Dev

- **Requires nightly Rust** — `edition = "2024"` + `build-std = ["std"]`.
  Switch: `rustup override set nightly`.
- `.cargo/config.toml` overrides global build flags (`+crt-static`, `opt-level=3`, `debuginfo=0`).
- Build: `cargo build` / Run: `cargo run` / Test: `cargo test`.
- Prefix shell commands with `rtk` for token compression (per `.clinerules`).

## Lints

- Clippy in `Cargo.toml`: `all`, `pedantic`, `nursery`, `unwrap_used` → `warn`.
- Rust lints: `unused`, `dead_code`, `unused_imports`, `unused_variables` → `warn`.
- Check: `cargo clippy`. No `rustfmt.toml` or `clippy.toml` — toolchain defaults used.

## Source Layout

| File | Role |
|------|------|
| `src/main.rs` | Entry point, `TrackingAllocator` (MiMalloc), `eframe::run_native`, `allocated_bytes()` |
| `src/app.rs` | `MyApp` (wires `Document` + `ToolConfig` + `UndoHistory` + `FileIO` + `UIState`), app identity constants, `ExportInfo`, `EXPORT_FORMATS` (13 formats), `UIState` (render state / autosave tracking), async autosave loop (2min interval) |
| `src/document.rs` | `Document` — wraps `Canvas` + save path + current layer; `render_to_texture()`, `add_layer()` / `delete_layer()` / `move_layer_up/down()` / `select_layer()`, `replace_canvas()` |
| `src/canvas.rs` | `Canvas`, `Layer`, `draw_square()`, `draw_square_line()`, `draw_circle()`, `draw_circle_line()`, `CurrentTool` enum (`Square` / `Circle` / `SquareEraser` / `CircleEraser`), `RenderState` enum (`ActiveWake` / `IdleThrottled` / `UnfocusedFrozen`) |
| `src/pixel.rs` | SIMD (`wide::u32x4`) + rayon parallel blend, premultiplied-alpha, `blend_layers()`, `unpremultiply()` |
| `src/files.rs` | `save_canvas()`, `load_canvas()`, `compress_canvas()`, `decompress_canvas()`, `save_compressed()`, `export_as_image()` — zstd-compressed JSON → `.splattercanvas` |
| `src/file_io.rs` | `FileIO` (async file dialogs via mpsc channels), `PendingFileAction`, `SaveKind`, `SaveResult`, autosave to `{data_dir}/autosaves/` |
| `src/undo.rs` | `UndoRecord`, `StrokePixel`, `RunSegment`, `BeforePixels` — per-pixel stroke apply / undo / redo application |
| `src/undo_history.rs` | `UndoHistory` — undo/redo stack with visited-stamp dedup (`MAX_STROKE_STACK = 1000`), `push_undo()` / `undo_step()` / `redo_step()` / `next_stamp()` |
| `src/tool_config.rs` | `ToolConfig` — `current_tool`, `current_color`, `radius`, `show_brush_preview`, `undo_redo_steps_multiplier` |
| `src/ui/` | 4 panels: `top` (file menu), `left` (tools), `right` (color / layers), `center` (canvas) |
| `src/tests/` | 6 test modules: `pixel`, `undo`, `undo_history`, `canvas`, `document`, `files` |

## Notable

- File format: `serde_json` → `zstd` level 10 → `.splattercanvas`
- Circle brush primitives supported (fill + stamp line)
- Async file IO via mpsc channels; export 13 image formats (AVIF, PNG, JPEG, WebP, GIF, TIFF, TGA, ICO, PNM, QOI, EXR, HDR, Farbfeld)
- No CI, no Makefile/justfile, no test harness
- Dev profile: `overflow-checks = true`, `incremental = true`, `codegen-units = 512`, `opt-level = 1`
- Release: `lto = "fat"`, `strip = true`, `panic = "abort"`
