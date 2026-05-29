# SplatterIron — Agent Guide

## Build & Dev

- **Requires Rust ≥1.96.0** (stable) — commit `rust-toolchain.toml` pins the channel.
- Build: `cargo build` / Run: `cargo run` / Test: `cargo test`.
- Prefix shell commands with `rtk` for token compression (per `.clinerules`).

## Lints

- Clippy in `Cargo.toml`: `all`, `pedantic`, `nursery`, `unwrap_used` → `warn`.
- Rust lints: `unused`, `dead_code`, `unused_imports`, `unused_variables` → `warn`.
- Check: `cargo clippy`.
- `clippy.toml`: `msrv = "1.96.0"`, `too-many-arguments-threshold = 9`, custom `disallowed-names`.
- `rustfmt.toml`: `edition = "2024"`, `max_width = 100`, `imports_granularity = "Item"`, `group_imports = "StdExternalCrate"`.

## Source Layout

| File                  | Role                                                                                                                                                                                                                                                 |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/main.rs`         | Entry point, `TrackingAllocator` (MiMalloc), `eframe::run_native`, `allocated_bytes()`                                                                                                                                                               |
| `src/app.rs`          | `MyApp` (wires `Document` + `ToolConfig` + `UndoHistory` + `FileIO` + `UIState`), app identity constants, `ExportInfo`, `EXPORT_FORMATS` (13 formats), `UIState` (render state / autosave tracking), async autosave loop (2min interval)             |
| `src/document.rs`     | `Document` — wraps `Canvas` + save path + current layer; `render_to_texture()`, `add_layer()` / `delete_layer()` / `move_layer_up/down()` / `select_layer()`, `replace_canvas()`                                                                     |
| `src/canvas.rs`       | `Canvas`, `Layer`, `draw_square()`, `draw_square_line()`, `draw_circle()`, `draw_circle_line()`, `CurrentTool` enum (`Square` / `Circle` / `SquareEraser` / `CircleEraser`), `RenderState` enum (`ActiveWake` / `IdleThrottled` / `UnfocusedFrozen`) |
| `src/pixel.rs`        | SIMD (`wide::u32x4`) + rayon parallel blend, premultiplied-alpha, `blend_layers()`, `unpremultiply()`                                                                                                                                                |
| `src/files.rs`        | `save_canvas()`, `load_canvas()`, `compress_canvas()`, `decompress_canvas()`, `save_compressed()`, `export_as_image()` — zstd-compressed JSON → `.splattercanvas`                                                                                    |
| `src/file_io.rs`      | `FileIO` (async file dialogs via mpsc channels), `PendingFileAction`, `SaveKind`, `SaveResult`, autosave to `{data_dir}/autosaves/`                                                                                                                  |
| `src/undo.rs`         | `UndoRecord`, `StrokePixel`, `RunSegment`, `BeforePixels` — per-pixel stroke apply / undo / redo application                                                                                                                                         |
| `src/undo_history.rs` | `UndoHistory` — undo/redo stack with visited-stamp dedup (`MAX_STROKE_STACK = 1000`), `push_undo()` / `undo_step()` / `redo_step()` / `next_stamp()`                                                                                                 |
| `src/tool_configuration.rs`  | `ToolConfiguration` — `current_tool`, `current_color`, `radius`, `alpha_overlay`, `show_brush_preview`, `undo_redo_steps_multiplier`                                                                     |
| `src/ui/`             | 4 panels: `top` (file menu), `left` (tools), `right` (color / layers), `center` (canvas)                                                                                                                                                             |
| `src/tools/`          | `bucket_fill` (scanline flood-fill), `circle_brush` (midpoint-circle span fill), `square_brush` (rectangular fill) — all return `UndoRecord`, used by `Document`/`Canvas`                                                                            |
| `src/tests/`          | 9 test modules: `pixel`, `undo`, `undo_history`, `canvas`, `document`, `files`, `bucket_fill`, `circle_brush`, `square_brush`                                                                                                                        |
| `docs/src/`           | Mirrors `src/` structure; one `.md` per `.rs` file for post-implementation documentation                                                                                                                                                             |
| `docs/architecture/`  | Numbered ADRs for deliberate architecture decisions                                                                                                                                                                                                  |

## Notable

- File format: `serde_json` → `zstd` level 10 → `.splattercanvas`
- Circle brush and square brush primitives supported (fill + stamp line); bucket fill (scanline flood-fill)
- Async file IO via mpsc channels; export 13 image formats (AVIF, PNG, JPEG, WebP, GIF, TIFF, TGA, ICO, PNM, QOI, EXR, HDR, Farbfeld)
- No CI, no Makefile/justfile, no test harness
- Dev profile: `overflow-checks = true`, `incremental = true`, `codegen-units = 512`, `opt-level = 1`
- Release: `lto = "fat"`, `strip = true`, `panic = "abort"`

## Project Philosophy

### Core Values

| Value                            | How codebase reflects it                                                                                                                            |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Performance first**            | SIMD (`wide::u32x4`) + rayon parallel blend, `MiMalloc` with `TrackingAllocator`, release `lto="fat"`, zstdmt compression                           |
| **Correctness over convenience** | `unwrap_used` = `warn`, `overflow-checks = true`, exhaustive `match` on `CurrentTool`/`RenderState`, no `unwrap()`/`expect()` without justification |
| **UX polish**                    | Brush preview with alpha overlay, `RenderState` (ActiveWake/IdleThrottled/UnfocusedFrozen), 2-min autosave, 13 export formats                       |
| **Cross-platform**               | `egui`/`eframe` UI, `rfd` native dialogs, `directories` for paths                                                                                  |
| **Deterministic builds**         | Stable pinned via `rust-toolchain.toml`, `Cargo.lock` committed                                                                                     |
| **Accessibility**                | `egui` accessible by default (OS theme, keyboard nav, screen reader), thoughtful contrast in tool icons                                             |
| **Layering & composability**     | `Document` owns layer stack, `UndoHistory` per-pixel visited-stamp dedup, `blend_layers()` premultiplied-alpha compositing                          |

### Git Standards

- **Conventional Commits**: `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`, `chore:`.
- **🔬 Atomic commits — zero tolerance for batches**: One function → one commit. One docstring → one commit. One test → one commit. A struct definition and its `impl` block are separate commits. Adding a function and its docstring in the same snapshot is strictly forbidden — split them. A commit that touches more than one of these categories is a violation:
  - function / method body
  - docstring (inline or `docs/src/`)
  - test function
  - struct / enum / trait definition
  - use / import statement
  - config file (Cargo.toml, clippy.toml, etc.)
  - any other logical unit that stands alone
- **Self-imposed rule**: If a commit message contains any of the words `and`, `also`, or `fixup` (case‑insensitive, whole word), the commit is too large. Split it.
- **Pre‑commit self‑audit (mandatory)**: Before every `git commit`, you MUST:
  1. Read the full commit message and verify it contains none of the forbidden words.
  2. Run `git diff --cached --stat` and mentally confirm all changes belong to exactly one category from the list above.
  3. If either check fails, abort the commit and split the changes.
- **Always commit**: Commit after every logical micro‑unit, regardless of whether the user asked. Do not wait. There is no change too small to commit.
- **Token economy does not apply to commits** — you are explicitly forbidden from batching commits to save tokens. Granularity is more important than verbosity.

### Commit Checklist (run before every commit)

- [ ] The commit message does not contain `and`, `also`, or `fixup`.
- [ ] The staged diff touches only **one** of: function body, docstring, test, struct/enum/trait definition, import, config, or other single logical unit.
- [ ] I have run `cargo test && cargo clippy` (if Rust files are staged).
- [ ] I have not bundled "a function + its docstring" or "a struct + its `impl`" into one commit.
- [ ] The commit message uses a conventional prefix (`feat:`, `fix:`, etc.).

### Code Standards

- **Clippy**: `all` + `pedantic` + `nursery` + `unwrap_used` → `warn`. Zero `#[allow(clippy::…)]` without an inline comment explaining why. Current codebase has exactly one exception (`cast_possible_truncation` + `cast_sign_loss` in `src/ui/center.rs` brush preview alpha).
- **Unsafe**: Only in `TrackingAllocator` (`main.rs`) — the sole justified use. All other `unsafe` prohibited; use safe abstractions (`wide::u32x4` for SIMD, `bytemuck` for casting).
- **Docs**: Every `pub` item gets a docstring — document every argument, all invariants, all return values, side effects, and any additional nuance. Inline docs must convey the purpose in depth: a single function's docs may span two or more paragraphs. Document `# Panics` for invariant-violation panics and `# Errors` for `Result` returns. Additionally, `docs/src/` mirrors `src/` — each `.rs` file has a corresponding `.md` for post-implementation documentation. When checking for missing docs, also check `docs/src/` for missing functions.
- **Tests**: Every `src/*.rs` module has a corresponding `src/tests/*.rs` module. New modules must add test coverage. Pre-commit gate: `cargo test && cargo clippy`.
- **Error handling**: Panic on invariant violations (logic bugs) with documented `# Panics`. `Result` for recoverable errors (IO, deserialization, dialogs) with documented `# Errors`.

### Agent Expectations

- Before committing, perform the Commit Checklist exactly as written in the Git Standards section. If any box is unchecked, stop and split the commit.
- All functions and structs MUST have an inline docstring. `docs/src/` files are post-implementation supplements — they are also required, but the inline doc comes first.
- Before editing a file, read surrounding context to match conventions.
- **Commit after every logical change — always. Do not wait for the user to ask.** Each function, each docstring, each test gets its own commit. If a commit message contains "and", "also", or "fixup", split it.
- Before committing, always run `cargo test && cargo clippy`.
- During planning mode, use the `question` tool frequently to gather preferences and clarify intent before implementing.
- If adding a new module, create a corresponding test module and register it in `src/tests/mod.rs`.
- Never suppress clippy lints without an inline justification comment.
