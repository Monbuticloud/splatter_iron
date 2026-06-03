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

(Change)(PX)(BX)(Time when TODO was made, use latest made or updated commit (use hash))

## Ideas

### Performance

- `compress_run` allocates `Vec<Color32>` before RLE check — `src/undo.rs:52` → defer allocation via iterator/callback. `stamp_brush` uses `mem::take` but callers still allocate upfront. (P2)(B1)(5d89e87)
- bucket fill lacks visited-stamp dedup — `src/tools/bucket_fill.rs:34` → inline stamp check (not `apply_visited_runs`; architectural mismatch). (P2)(B2)(5d89e87)
- grid overlay redraws all lines every frame — `src/ui/center.rs:120-148` → cache as shape or vertex buffer. (P3)(B1)(5d89e87)
- adaptive render quality — reduce blend resolution when zoomed far out. (P3)(B1)(5d89e87)
- ~~`stamp_circle_positions` uses `f64::sqrt` per row — implemented: integer midpoint-circle~~ (P1)(B1)(5d89e87)
- ~~square brush `stamp_line_positions` stamps full (2R)² rect — implemented: per-row span deduplication~~ (P1)(B1)(5bf5fc9)
- ~~`blend_region` recomputes `base_indices` — cancelled: overhead ~100ns, not worth caching~~ (P2)(B1)(5bf5fc9)
- ~~`redo_apply` alpha overlay path — implemented: SIMD-vectorized with `wide::u32x4`~~ (P2)(B1)(5bf5fc9)
- ~~bucket fill stack grows unbounded — implemented: added upper bound~~ (P2)(B1)(5bf5fc9)

### Architecture

- split `FileIO` — `src/file_io.rs:99` manages dialog/save/export/load/import/stamp/brush → `DialogManager`, `SaveManager`, `ExportManager`, `ImportManager`. (P1)(B5)(5d89e87)
- `apply_stroke` duplicates `BrushStrokeParams` builder construction — `src/ui/center.rs:644-878` → builder extracted but `::builder(...)` call repeated 6×; hoist invariant args above match. (P1)(B1)(5d89e87)
- consolidate eraser variants — explore `ToolKind { Square, Circle }` + `Eraser(ToolKind)` layout (enum, not bool). (P2)(B4)(5d89e87)
- deduplicate stamp/brush fields in `ToolConfiguration` — share sub-struct for `sampling`/`tint_mode`. (P2)(B3)(5d89e87)
- layer blend modes — add Multiply, Screen, Overlay etc. (currently only alpha-overlay + opaque). (P2)(B2)(5d89e87)
- selection tools — rectangular/lasso/magic-wand. (P2)(B2)(5d89e87)
- layer locking — add `alpha_lock` / `full_lock` fields to `Layer`, wire into blend and brush-apply. (P2)(B2)(5d89e87)
- canvas rotation/flip — store rotation/filp in `UIState`; apply view transform in `center.rs`. (P2)(B2)(5d89e87)
- line tool — click-drag straight line; preview during drag; shift-constrain to 45°. (P2)(B2)(5d89e87)
- canvas background checkerboard — blend behind transparent areas in `blend_layers()`. (P3)(B2)(5d89e87)
- rectangle/ellipse shape tools — unfilled/stroked shapes with configurable border width. (P3)(B2)(5d89e87)
- layer-snapshot undo for >50% coverage strokes — store zstd-compressed full layer clone instead of per-pixel before-data. Simplifies undo path for large strokes, avoids per-segment overhead. (P2)(B2)(9d11f23)
- ~~drag accumulator has no max frame limit — implemented: `MAX_DRAG_FRAMES=5000` with auto-finalize~~ (P2)(B1)(5bf5fc9)
- ~~alpha overlay rounding drift — cancelled: undo stores exact before-pixels, redo always blends from same base state, no drift~~ (P2)(B1)(5bf5fc9)
- ~~memory warning estimate ignores actual layer count — implemented: `estimate_canvas_memory` now takes `layer_count` parameter~~ (P2)(B1)(5bf5fc9)

### UX

- keyboard shortcut system with help dialog — top-bar "Keyboard Shortcuts" / `?` button. (P2)(B2)(5d89e87)
- pressure sensitivity — `pressure: f32` in `BrushStrokeParams` (per ADR-0018). (P3)(B2)(5d89e87)
- brush radius keyboard shortcuts — `[`/`]` decrease/increase radius; `Shift+[`/`]` fine adjustment. (P2)(B2)(5d89e87)
- status bar — dimensions + zoom + activity shown; missing cursor coordinates + memory usage. (P2)(B2)(5d89e87)
- persist UI state — tool config + recent files persisted; missing window size, panel widths, zoom, pan offset, last export format. (P2)(B2)(5d89e87)
- color palette/swatches — save/load/recent colors panel, persisted to disk. (P3)(B2)(5d89e87)
- fullscreen toggle — `Cmd+Ctrl+F` or menu entry. (P3)(B2)(5d89e87)
- panel visibility toggles — `Tab` to toggle all panels, individual show/hide buttons. (P3)(B2)(5d89e87)
- preferences/settings dialog — default canvas size, autosave interval, theme. (P3)(B2)(5d89e87)

### Standards & Cleanup

- ~~missing `# Errors` doc sections — `parse_gbr` and `parse_abr` now have `# Errors`; remaining 5 private fns exempt per standards~~ (P2)(B1)(5d89e87)
- unused imports — `src/file_io.rs:4` (`Path`), `src/tools/custom_brush.rs:9` (`Canvas`) — 2 of 4 remain; `debug.rs:7`, `dialogs.rs:13` fixed. (P3)(B1)(5d89e87)
- dead code audit — `#[cfg(test)]` gate or remove 21 dead items in `asset_library`, `canvas`, `files`, `undo_history`. (P3)(B1)(5d89e87)
- scalar head/tail blending logic duplicated 3× in `apply_single_layer` — `src/pixel.rs:350-483` → extract into shared closure to eliminate copy-paste. (P3)(B1)(5bf5fc9)
- ~~stale `compress_run` name in module docstring — fixed~~ (P4)(B1)(5bf5fc9)

### Testing

- ~~missing `src/tests/frame.rs` — created, migrated 4 inline tests~~ (P2)(B1)(5d89e87)
