# TODO

## Priority levels

- (P0) - Critical
- (P1) - High
- (P2) - Medium
- (P3) - Minor
- (P4) - Trivial
- (P5) - Nice to have

## Change level

- (B0) - No impact
- (B1) - Internal only
- (B2) - Compatible change
- (B3) - Deprecated interface
- (B4) - Breaking interface change
- (B5) - Architectural overhaul

## Format

(Change)(PX)(BX)(Time when TODO was made, use latest commit (use hash))

## Ideas

### Performance

- `export_as_image` pixel-by-pixel loop — `src/files.rs:247` → use `bytemuck` cast + parallel iteration. (P1)(B1)(aef7235)
- `compress_run` allocates `Vec<Color32>` before RLE check — `src/undo.rs:52` → defer allocation via iterator/callback. (P2)(B1)(aef7235)
- bucket fill lacks visited-stamp dedup — `src/tools/bucket_fill.rs:34` → reuse `apply_visited_runs`. (P2)(B2)(aef7235)
- `stamp_circle_positions` uses `f64::sqrt` per row — `src/tools/circle_brush.rs:312` → integer midpoint-circle increment. (P2)(B1)(aef7235)
- grid overlay redraws all lines every frame — `src/ui/center.rs:120-148` → cache as shape or vertex buffer. (P3)(B1)(aef7235)
- adaptive render quality — reduce blend resolution when zoomed far out. (P3)(B1)(59653a1)

### Architecture

- split `FileIO` — `src/file_io.rs:99` manages dialog/save/export/load/import/stamp/brush → `DialogManager`, `SaveManager`, `ExportManager`, `ImportManager`. (P1)(B5)(aef7235)
- `apply_stroke` duplicates `BrushStrokeParams` construction — `src/ui/center.rs:509-751` → extract builder. (P1)(B1)(aef7235)
- consolidate eraser variants — explore `ToolKind { Square, Circle }` + `Eraser(ToolKind)` layout (enum, not bool). (P2)(B4)(aef7235)
- deduplicate stamp/brush fields in `ToolConfiguration` — share sub-struct for `sampling`/`tint_mode`. (P2)(B3)(aef7235)
- layer blend modes — add Multiply, Screen, Overlay etc. (currently only alpha-overlay + opaque). (P2)(B2)(aef7235)
- selection tools — rectangular/lasso/magic-wand. (P2)(B2)(aef7235)
- layer locking — add `alpha_lock` / `full_lock` fields to `Layer`, wire into blend and brush-apply. (P2)(B2)(59653a1)
- canvas rotation/flip — store rotation/filp in `UIState`; apply view transform in `center.rs`. (P2)(B2)(59653a1)
- line tool — click-drag straight line; preview during drag; shift-constrain to 45°. (P2)(B2)(59653a1)
- canvas background checkerboard — blend behind transparent areas in `blend_layers()`. (P3)(B2)(59653a1)
- rectangle/ellipse shape tools — unfilled/stroked shapes with configurable border width. (P3)(B2)(59653a1)

### UX

- keyboard shortcut system with help dialog — top-bar "Keyboard Shortcuts" / `?` button. (P2)(B2)(aef7235)
- pressure sensitivity — `pressure: f32` in `BrushStrokeParams` (per ADR-0018). (P3)(B2)(aef7235)
- brush radius keyboard shortcuts — `[`/`]` decrease/increase radius; `Shift+[`/`]` fine adjustment. (P2)(B2)(59653a1)
- status bar — canvas dimensions, zoom %, cursor coordinates, memory usage at window bottom. (P2)(B2)(59653a1)
- persist UI state — save/restore window size, panel widths, zoom, pan offset, last export format in `PersistedConfig`. (P2)(B2)(59653a1)
- color palette/swatches — save/load/recent colors panel, persisted to disk. (P3)(B2)(59653a1)
- fullscreen toggle — `Cmd+Ctrl+F` or menu entry. (P3)(B2)(59653a1)
- panel visibility toggles — `Tab` to toggle all panels, individual show/hide buttons. (P3)(B2)(59653a1)
- preferences/settings dialog — default canvas size, autosave interval, theme. (P3)(B2)(59653a1)

### Standards & Cleanup

- `#[allow(clippy::..)]` missing inline justification — `src/ui/center.rs:296`, `src/tools/stamp_brush.rs:85` — add inline comment per AGENTS.md. (P1)(B1)(59653a1)
- `unwrap()` calls without justification — `src/ui/dialogs.rs:224,411` — add safety comment or replace with `if let`. (P1)(B1)(59653a1)
- missing `# Errors` doc sections — `src/tools/brush_parsers.rs:72,158,256,316,340,359,390` — 7 `Result` fns lacking `# Errors`. (P2)(B1)(59653a1)
- misplaced docstring — `src/app/mod.rs:75-78`: `/// File-extension list...` attributed to `PersistedConfig` instead of `IMPORT_EXTENSIONS`. (P2)(B1)(59653a1)
- duplicate docstring — `src/ui/center.rs:40-46` — first sentence appears twice. (P3)(B1)(59653a1)
- unused imports — `src/debug.rs:7`, `src/file_io.rs:4`, `src/tools/custom_brush.rs:9`, `src/ui/dialogs.rs:13`. (P3)(B1)(59653a1)
- dead code audit — remove or `#[cfg(test)]` gate 10+ dead items in `asset_library`, `canvas`, `files`, `undo_history`. (P3)(B1)(59653a1)

### Testing

- missing test module — `src/persistence.rs` has no `src/tests/persistence.rs`. (P1)(B1)(59653a1)
- missing frame.rs tests — `src/app/frame.rs` (GPU sync, render state, autosave) has zero coverage. (P2)(B1)(59653a1)
- missing `docs/src/tests/` files — `asset_library.md`, `brush_common.md`, `brush_params.md`, `debug.md`. (P3)(B0)(59653a1)

### Documentation

- README is sparse — add build instructions, feature list, screenshot, contribution guide. (P3)(B0)(aef7235)

## Accepted

-

## Implementing

-
