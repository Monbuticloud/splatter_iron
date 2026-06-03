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

## Rules

- **Preserve empty sections** — do not delete a `##` section from TODO.md just because it currently has no items. Empty sections act as placeholders for future work and signal intentional gaps.
- **Promote active work** — when implementation of a TODO item begins, move it from its current section (e.g. Ideas, Accepted) into the `## Implementing` section.
- **Archive on completion** — once implementation finishes, move the item from `## Implementing` to TODO_ARCHIVE.md under the appropriate `## Implemented` or `## Denied` section.

## Ideas

### Performance

- adaptive render quality — reduce blend resolution when zoomed far out. (P3)(B1)(5d89e87)
- reduce undo zstd compression level from default to 0 for 5x faster compression on main thread. (P1)(B1)(24f670e)
- avoid full layer pixel clone in `AddLayer` undo record — store placeholder that recreates transparent pixels on undo instead of cloning entire `Layer`. (P2)(B1)(24f670e)
- optimize `stamp_circle_positions` inner loops — use midpoint-circle span filling (like `fill_circle_impl`) instead of pixel-by-pixel iteration per Bresenham step. (P3)(B1)(24f670e)
- refactor duplicated `read_canvas`/`read_canvas_xz` and `write_canvas`/`write_canvas_xz` into generic functions accepting a decoder/encoder factory. (P3)(B1)(24f670e)

### Security

### Architecture

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
- add `Canvas` resize guard for undo records — version-stamp run segments against canvas dimensions so undo/redo are no-ops after resize. (P3)(B1)(24f670e)
- create ADR-0025: Error Handling Strategy — document panic-vs-Result philosophy with ADR template. (P3)(B0)(24f670e)

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

### Documentation

- fix `docs/src/file_io.md` — `push_recent_file` is in `persistence.rs`, not the `file_io` module; clarify the delegation chain. (P3)(B1)(24f670e)
- add `#[cfg(test)]` annotation notice to `docs/src/files.md` for `save_canvas_to_bytes`/`load_canvas_from_bytes` — they're test-only. (P3)(B1)(24f670e)
- create consolidated "Performance Architecture" document from ADR-0001, 0004, 0005, 0010, 0012 — centralized perf strategy reference. (P3)(B0)(24f670e)
- create `.splattercanvas` file format specification document with exact JSON schema. (P4)(B0)(24f670e)
- expand `docs/src/ui/panels.md` (556B) to cover panel visibility toggle details and usage. (P3)(B1)(24f670e)
- expand `docs/src/app/frame.md` (936B) to fully document frame lifecycle (poll, render state, GPU sync, autosave). (P3)(B1)(24f670e)

### Standards & Cleanup

- dead code audit — `#[cfg(test)]` gate or remove 21 dead items in `asset_library`, `canvas`, `files`, `undo_history`. (P3)(B1)(5d89e87)

## Accepted

### Performance

- add image import dimension limits (max 16384×16384 / 50MP) to prevent OOM from malicious images. (P1)(B1)(24f670e)
- increase `PARALLEL_BLEND_THRESHOLD` from 64 to 256 to reduce rayon overhead on small dirty rects. (P2)(B1)(24f670e)
- use capacity-based reallocation for `output_rgba` in `blend_to_output` instead of exact-length check to avoid realloc when capacity suffices. (P2)(B1)(24f670e)
- eliminate intermediate `RgbaImage` allocation for all export formats (PNG, WebP, TIFF, TGA, PNM, QOI) — currently only JPEG/HDR/Farbfeld skip it. (P2)(B1)(24f670e)
- stream JPEG RGB output to avoid intermediate `Vec<u8>` allocation — write unpremultiplied RGB directly via pre-allocated buffer. (P2)(B1)(24f670e)
- use `alpha_blend_simd_four` in alpha-overlay brush paths (`bucket_fill.rs`, `circle_brush.rs`) — currently uses scalar `alpha_blend` per pixel. (P2)(B1)(24f670e)
- remove dead/commented dependencies from `Cargo.toml`: `tokio`, `rand`, `target`, `type-layout`. (P3)(B1)(24f670e)

### Security

- add config file size limit (1MB `Read::take`) for `serde_json::from_reader` to prevent malformed config OOM. (P2)(B1)(24f670e)
- add canvas dimension validation against `max_texture_dimension_2d` at creation and import time. (P2)(B1)(24f670e)

### Documentation

- add `Superseded-By: ADR-0024` marker to ADR-0006 and ADR-0021 front matter (flat buffer + zstd partially replaced their storage model). (P2)(B1)(24f670e)

## Implementing

- None at the moment
