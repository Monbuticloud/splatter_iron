# SplatterIron — Agent Guide

## Build & Dev

- **Nightly Rust** — `edition = "2024"` + `build-std = ["std"]`.
  Switch: `rustup override set nightly`.
- `.cargo/config.toml` override global build flags (`+crt-static`, `opt-level=3`, `debuginfo=0`).
- Build: `cargo build` / Run: `cargo run` / Test: `cargo test`.
- Prefix shell commands with `rtk` for token compression (per `.clinerules`).

## Lints

- Clippy in `Cargo.toml`: `all`, `pedantic`, `nursery`, `unwrap_used` → `warn`.
- Rust lints: `unused`, `dead_code`, `unused_imports`, `unused_variables` → `warn`.
- Check: `cargo clippy`.
- `clippy.toml`: `msrv = "1.85.0"`, `too-many-arguments-threshold = 9`, custom `disallowed-names`.
- `rustfmt.toml`: `edition = "2024"`, `max_width = 100`, `imports_granularity = "Item"`, `group_imports = "StdExternalCrate"`.

## Source Layout

| File                       | Role                                                                                                    |
| -------------------------- | ------------------------------------------------------------------------------------------------------- |
| `src/main.rs`              | Entry point, `TrackingAllocator` (MiMalloc), `eframe::run_native`, `allocated_bytes()`                  |
| `src/app.rs`               | `MyApp` (wires `Document` + `ToolConfig` + `UndoHistory` + `FileIO` + `UIState`), app identity, `ExportInfo`, `EXPORT_FORMATS` (13), `UIState`, autosave loop (2min) |
| `src/document.rs`          | `Document` — wraps `Canvas` + save path + current layer; `render_to_texture()`, `add_layer()` / `delete_layer()` / `move_layer_up/down()` / `select_layer()`, `replace_canvas()` |
| `src/canvas.rs`            | `Canvas`, `Layer`, `draw_square()`, `draw_square_line()`, `draw_circle()`, `draw_circle_line()`, `CurrentTool` enum (`Square` / `Circle` / `SquareEraser` / `CircleEraser`), `RenderState` enum (`ActiveWake` / `IdleThrottled` / `UnfocusedFrozen`) |
| `src/pixel.rs`             | SIMD (`wide::u32x4`) + rayon parallel blend, premultiplied-alpha, `blend_layers()`, `unpremultiply()`   |
| `src/files.rs`             | `save_canvas()`, `load_canvas()`, `compress_canvas()`, `decompress_canvas()`, `save_compressed()`, `export_as_image()` — zstd-compressed JSON → `.splattercanvas` |
| `src/file_io.rs`           | `FileIO` (async file dialogs via mpsc), `PendingFileAction`, `SaveKind`, `SaveResult`, autosave to `{data_dir}/autosaves/` |
| `src/undo.rs`              | `UndoRecord`, `StrokePixel`, `RunSegment`, `BeforePixels` — per-pixel stroke apply / undo / redo        |
| `src/undo_history.rs`      | `UndoHistory` — undo/redo stack with visited-stamp dedup (`MAX_STROKE_STACK = 1000`), `push_undo()` / `undo_step()` / `redo_step()` / `next_stamp()` |
| `src/tool_configuration.rs` | `ToolConfiguration` — `current_tool`, `current_color`, `radius`, `alpha_overlay`, `show_brush_preview`, `undo_redo_steps_multiplier` |
| `src/ui/`                  | 4 panels: `top` (file menu), `left` (tools), `right` (color / layers), `center` (canvas)               |
| `src/tools/`               | `bucket_fill` (scanline flood-fill), `circle_brush` (midpoint-circle span fill), `square_brush` (rect fill) — all return `UndoRecord` |
| `src/tests/`               | 9 test modules: `pixel`, `undo`, `undo_history`, `canvas`, `document`, `files`, `bucket_fill`, `circle_brush`, `square_brush` |
| `docs/src/`                | Mirrors `src/`; one `.md` per `.rs` file for post-implementation docs                                  |
| `docs/architecture/`       | Numbered ADRs for deliberate architecture decisions                                                     |

## Notable

- File format: `serde_json` → `zstd` level 10 → `.splattercanvas`
- Circle + square brush primitives (fill + stamp line); bucket fill (scanline flood-fill)
- Async file IO via mpsc channels; export 13 image formats (AVIF, PNG, JPEG, WebP, GIF, TIFF, TGA, ICO, PNM, QOI, EXR, HDR, Farbfeld)
- No CI, no Makefile/justfile, no test harness
- Dev profile: `overflow-checks = true`, `incremental = true`, `codegen-units = 512`, `opt-level = 1`
- Release: `lto = "fat"`, `strip = true`, `panic = "abort"`

## Project Philosophy

### Core Values

| Value                            | How codebase reflects it                                                                    |
| -------------------------------- | ------------------------------------------------------------------------------------------- |
| **Performance first**            | SIMD (`wide::u32x4`) + rayon parallel blend, `MiMalloc` with `TrackingAllocator`, release `lto="fat"`, `opt-level=3`, zstdmt compression |
| **Correctness over convenience** | `unwrap_used` = `warn`, `overflow-checks = true`, exhaustive `match` on `CurrentTool`/`RenderState`, no bare `unwrap()`/`expect()` |
| **UX polish**                    | Brush preview with alpha overlay, `RenderState` (ActiveWake/IdleThrottled/UnfocusedFrozen), 2-min autosave, 13 export formats |
| **Cross-platform**               | `egui`/`eframe` UI, `rfd` native dialogs, `directories` for paths, Zig `compiler_rt` in `lib/` for cross-compilation |
| **Deterministic builds**         | Nightly pinned via `rustup override`, `build-std = ["std"]`, `Cargo.lock` committed         |
| **Accessibility**                | `egui` accessible by default (OS theme, keyboard nav, screen reader), thoughtful contrast in tool icons |
| **Layering & composability**     | `Document` owns layer stack, `UndoHistory` per-pixel visited-stamp dedup, `blend_layers()` premultiplied-alpha compositing |

### Git Standards

- **Conventional Commits**: `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`, `chore:`.
- **🔬 Atomic commits — zero tolerance for batches**: One function → one commit. One docstring → one commit. One test → one commit. Struct definition + `impl` block = separate commits. Function + docstring in same snapshot = forbidden. A commit touching >1 category is violation:
  - function / method body
  - docstring (inline or `docs/src/`)
  - test function
  - struct / enum / trait definition
  - use / import statement
  - config file (Cargo.toml, clippy.toml, etc.)
  - any other standalone logical unit
- **Self-imposed rule**: If commit message contains `and`, `also`, or `fixup` (case-insensitive, whole word), commit too large. Split.
- **Pre-commit self-audit (mandatory)**: Before every `git commit`:
  1. Read full commit message — verify no forbidden words.
  2. Run `git diff --cached --stat` — confirm changes belong to exactly one category.
  3. If either fails, abort + split.
- **Always commit**: After every logical micro-unit. No change too small.
- **Token economy does not apply to commits** — batching commits to save tokens is forbidden. Granularity > verbosity.

### Commit Checklist (run before every commit)

- [ ] Commit message does not contain `and`, `also`, or `fixup`.
- [ ] Staged diff touches only **one** of: function body, docstring, test, struct/enum/trait definition, import, config, or other single logical unit.
- [ ] Run `cargo test && cargo clippy` (if Rust files staged).
- [ ] Did not bundle "function + docstring" or "struct + `impl`" into one commit.
- [ ] Commit message uses conventional prefix (`feat:`, `fix:`, etc.).

### Code Standards

- **Clippy**: `all` + `pedantic` + `nursery` + `unwrap_used` → `warn`. Zero `#[allow(clippy::…)]` without inline justification. Current codebase has one exception (`cast_possible_truncation` + `cast_sign_loss` in `src/ui/center.rs` brush preview alpha).
- **Unsafe**: Only in `TrackingAllocator` (`main.rs`). All other `unsafe` prohibited; use safe abstractions (`wide::u32x4` for SIMD, `bytemuck` for casting).
- **Docs**: Every `pub` item gets docstring — document all args, invariants, return values, side effects, nuance. Inline docs may span 2+ paragraphs. Document `# Panics` for invariant-violation panics, `# Errors` for `Result` returns. `docs/src/` mirrors `src/` — each `.rs` file has corresponding `.md`. Check `docs/src/` for missing functions too.
- **Tests**: Every `src/*.rs` module has corresponding `src/tests/*.rs` module. New modules must add test coverage. Pre-commit gate: `cargo test && cargo clippy`.
- **Error handling**: Panic on invariant violations (logic bugs) with documented `# Panics`. `Result` for recoverable errors (IO, deserialization, dialogs) with documented `# Errors`.

### Agent Expectations

- Before committing, run Commit Checklist exactly as written. If any box unchecked, stop + split.
- All functions and structs MUST have inline docstring. `docs/src/` files are post-implementation supplements — required, but inline doc comes first.
- Before editing a file, read surrounding context to match conventions.
- **Commit after every logical change — always. Do not wait.** Each function, docstring, test gets own commit. If commit message contains "and", "also", or "fixup", split.
- Before committing, run `cargo test && cargo clippy`.
- During planning mode, use `question` tool frequently to gather preferences + clarify intent.
- If adding new module, create corresponding test module + register in `src/tests/mod.rs`.
- Never suppress clippy lints without inline justification comment.
