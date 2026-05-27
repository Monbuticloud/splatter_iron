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

## Project Philosophy

### Core Values

| Value | How codebase reflects it |
|---|---|
| **Performance first** | SIMD (`wide::u32x4`) + rayon parallel blend, `MiMalloc` with `TrackingAllocator`, release `lto="fat"`, `opt-level=3`, zstdmt compression |
| **Correctness over convenience** | `unwrap_used` = `warn`, `overflow-checks = true`, exhaustive `match` on `CurrentTool`/`RenderState`, no `unwrap()`/`expect()` without justification |
| **UX polish** | Brush preview with alpha overlay, `RenderState` (ActiveWake/IdleThrottled/UnfocusedFrozen), 2-min autosave, 13 export formats |
| **Cross-platform** | `egui`/`eframe` UI, `rfd` native dialogs, `directories` for paths, Zig `compiler_rt` in `lib/` for cross-compilation |
| **Deterministic builds** | Nightly pinned via `rustup override`, `build-std = ["std"]`, `Cargo.lock` committed |
| **Accessibility** | `egui` accessible by default (OS theme, keyboard nav, screen reader), thoughtful contrast in tool icons |
| **Layering & composability** | `Document` owns layer stack, `UndoHistory` per-pixel visited-stamp dedup, `blend_layers()` premultiplied-alpha compositing |

### Git Standards

- **Conventional Commits**: `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`, `chore:`.
- Subject ≤50 chars; body explains "why" when the commit message alone is insufficient.
- **🔬 Ultra-granular commits**: One function → one commit. One docstring → one commit. One test → one commit. Another test → another commit. No commit shall contain more than one logical change. **There is no such thing as too many commits.** A commit that fixes a function and adds its docstring in the same snapshot is *too big* — split it. If you hesitate whether to commit, commit. Err on the side of granularity always.
- **Self-imposed rule**: If a commit message contains "and", "also", or "fixup", the commit is too large. Split it.

### Code Standards

- **Clippy**: `all` + `pedantic` + `nursery` + `unwrap_used` → `warn`. Zero `#[allow(clippy::…)]` without an inline comment explaining why. Current codebase has exactly one exception (`cast_possible_truncation` + `cast_sign_loss` in `src/ui/center.rs` brush preview alpha).
- **Unsafe**: Only in `TrackingAllocator` (`main.rs`) — the sole justified use. All other `unsafe` prohibited; use safe abstractions (`wide::u32x4` for SIMD, `bytemuck` for casting).
- **Docs**: Every `pub` item gets a docstring. Document `# Panics` for invariant-violation panics and `# Errors` for `Result` returns.
- **Tests**: Every `src/*.rs` module has a corresponding `src/tests/*.rs` module. New modules must add test coverage. Pre-commit gate: `cargo test && cargo clippy`.
- **Error handling**: Panic on invariant violations (logic bugs) with documented `# Panics`. `Result` for recoverable errors (IO, deserialization, dialogs) with documented `# Errors`.

### Agent Expectations

- Before editing a file, read surrounding context to match conventions.
- Before committing, always run `cargo test && cargo clippy`.
- During planning mode, use the `question` tool frequently to gather preferences and clarify intent before implementing.
- If adding a new module, create a corresponding test module and register it in `src/tests/mod.rs`.
- Never suppress clippy lints without an inline justification comment.
