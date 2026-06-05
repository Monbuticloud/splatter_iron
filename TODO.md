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

(Change)(PX)(BX)(Time when TODO was made, use latest made or updated commit (use hash), you are NOT allowed to use relative timestamps (e.g HEAD, Latest, 5 commits ago))

## Rules

- **Preserve empty sections** — do not delete a `##` section from TODO.md just because it currently has no items. Empty sections act as placeholders for future work and signal intentional gaps.
- **Promote active work** — when implementation of a TODO item begins, move it from its current section (e.g. Ideas, Accepted) into the `## Implementing` section.
- **Archive on completion** — once implementation finishes, move the item from `## Implementing` to TODO_ARCHIVE.md under the appropriate `## Implemented` or `## Denied` section.

## Ideas

### Performance

- adaptive render quality — reduce blend resolution when zoomed far out. (P3)(B1)(460008e)
- refactor duplicated `read_canvas`/`read_canvas_xz` and `write_canvas`/`write_canvas_xz` into generic functions accepting a decoder/encoder factory. (P3)(B1)(460008e)
- `compress_and_store` double-pass on uniform runs — `slice.iter().all()` then `extend_from_slice` reads data twice; skip redundant memcpy. (P4)(B1)(e91442e)
- `visited_stamp` wrap triggers O(pixels) fill on UI thread — reset entire visited buffer (3M+ entries) on u32 overflow, causes frame stutter. (P3)(B1)(e91442e)
- add `config_dirty` flag to `persistence.rs` — `handle_config_save` writes `config.json` every 2min tick even when no config changed. (P3)(B1)(b9eb27f)

### Security

- **(none at the moment)**

### Architecture

- layer blend modes — add Multiply, Screen, Overlay etc. (currently only alpha-overlay + opaque). (P2)(B2)(460008e)
- selection tools — rectangular/lasso/magic-wand. (P2)(B2)(460008e)
- layer locking — add `alpha_lock` / `full_lock` fields to `Layer`, wire into blend and brush-apply. (P2)(B2)(460008e)
- canvas rotation/flip — store rotation/filp in `UIState`; apply view transform in `center.rs`. (P2)(B2)(460008e)
- rectangle/ellipse shape tools — unfilled/stroked shapes with configurable border width. (P3)(B2)(460008e)
- create ADR-0025: Error Handling Strategy — document panic-vs-Result philosophy with ADR template. (P3)(B0)(460008e)
- `ClippedOverlap` `suppress_base` logic incorrect for >2-deep clip chains — nested clipping layers beyond 2 cause `suppress_base` to skip wrong `Normal` layers. (P3)(B1)(e91442e)
- extract model/view helper from `apply_stroke` match arms — 4× near-identical blocks (Square/Circle/Stamp/CustomBrush) each ~30 lines; hoist shared pattern. (P2)(B1)(b9eb27f)
- extract zoom/pan/grid code from 530-line `handle_canvas_interaction` into focused sub-functions (canvas rect, grid overlay, context menu, brush preview, etc.). (P1)(B1)(b9eb27f)
- deduplicate stamp/brush gallery rendering in `show_left_panel` — near-identical ~70-line blocks for asset gallery, tint mode, sampling dropdown. (P2)(B1)(b9eb27f)
- refactor `export_as_image` into per-format strategy structs — 252-line function with 5 special cases (JPEG/HDR/Farbfeld/ICO/GIF) + macro + fallback. (P2)(B1)(b9eb27f)
- provide default methods or derive macro for `AssetEntry` trait — reduces 11-method impl to 3-5 overridable defaults. (P3)(B2)(b9eb27f)
- split `UIState` (25 fields) into focused sub-structs — CanvasViewState, CursorState, TimelineState, OverlayState, PersistenceState, ToolUIState. (P2)(B3)(b9eb27f)

### UX

- keyboard shortcut system with help dialog — top-bar "Keyboard Shortcuts" / `?` button. (P2)(B2)(460008e)
- pressure sensitivity — `pressure: f32` in `BrushStrokeParams` (per ADR-0018). (P3)(B2)(460008e)
- color palette/swatches — save/load/recent colors panel, persisted to disk. (P3)(B2)(460008e)
- fullscreen toggle — `Cmd+Ctrl+F` or menu entry. (P3)(B2)(460008e)
- panel visibility toggles — `Tab` to toggle all panels, individual show/hide buttons. (P3)(B2)(460008e)
- preferences/settings dialog — default canvas size, autosave interval, theme. (P3)(B2)(460008e)

### Documentation

- fix `docs/src/file_io.md` — `push_recent_file` is in `persistence.rs`, not the `file_io` module; clarify the delegation chain. (P3)(B1)(460008e)
- add `#[cfg(test)]` annotation notice to `docs/src/files.md` for `save_canvas_to_bytes`/`load_canvas_from_bytes` — they're test-only. (P3)(B1)(460008e)
- create consolidated "Performance Architecture" document from ADR-0001, 0004, 0005, 0010, 0012 — centralized perf strategy reference. (P3)(B0)(460008e)
- create `.splattercanvas` file format specification document with exact JSON schema. (P4)(B0)(460008e)
- expand `docs/src/ui/panels.md` (556B) to cover panel visibility toggle details and usage. (P3)(B1)(460008e)
- expand `docs/src/app/frame.md` (936B) to fully document frame lifecycle (poll, render state, GPU sync, autosave). (P3)(B1)(460008e)
- fix `docs/src/pixel.md` stale function reference — `blend_pixel_range` was merged into `apply_single_layer`. (P3)(B1)(e91442e)
- document drag accumulator `MAX_DRAG_FRAMES` split edge case — when exceeded, creates separate undo records for one drag gesture. (P3)(B1)(e91442e)
- remove duplicate docstring block in `undo.rs:maybe_snapshot` — identical 30-line docstring appears twice (`///` lines 170-208 and 210-248). (P3)(B0)(b9eb27f)

### Standards & Cleanup

- fix clippy `as` cast warnings — prioritize `u32 as f32` precision-loss (40x) and `u32 as u8` truncation (33x) to surface real overflow bugs. (P2)(B1)(e91442e)
- `xz2` optional dependency — `.splatterarchive` format is export-only; remove if unused to cut C dep and build time. (P4)(B1)(e91442e)
- remove stale `# type-layout` comment from `Cargo.toml:29` — archived as already-removed but still present. (P4)(B0)(b9eb27f)
- fix "TrackingAllocator" reference in `main.rs` module docstring and `AGENTS.md`/`technical-domain.md` — `TrackingAllocator` wrapper was removed per ADR-0001; global allocator is plain `MiMalloc`. (P3)(B0)(b9eb27f)
- replace magic range `(0..64)` with dynamic focus-id count in `top.rs:34` — brush name focus detection silently fails if >64 fields. (P3)(B1)(b9eb27f)
- audit all ~20 `.expect()` calls — add inline justification comments or convert to `Result` (particularly `undo.rs`/`undo_history.rs` compression calls that can fail on disk-full). (P2)(B1)(b9eb27f)
- move `#[cfg(test)]` helper methods from production modules (`undo_history.rs`, `asset_library.rs`, `canvas.rs`, `files.rs`, `undo.rs`) into `src/tests/`. (P3)(B1)(b9eb27f)

## Accepted

### Performance

- avoid full layer pixel clone in `AddLayer` undo record — store placeholder that recreates transparent pixels on undo instead of cloning entire `Layer`. (P2)(B1)(460008e)
- `fill_square_impl` alpha-overlay scalar loop — use `alpha_blend_span` for SIMD 4× throughput on square alpha strokes. (P1)(B1)(e91442e)
- `stamp_at` allocates `before: Vec<Color32>` per row — hoist outside row loop and reuse via `clear()`. (P1)(B1)(e91442e)
- `apply_visited_runs` alpha-overlay scalar loop — use `alpha_blend_span` for SIMD 4× throughput on all tool drag strokes (shared by square/circle/stamp brushes). (P1)(B1)(e91442e)
- `stamp_circle_positions` inner loop stamps per-pixel O(L·R²) — add per-row span tracking (like square brush's `row_min_x`/`row_max_x`) for O(L·R). (P2)(B1)(e91442e)
- `draw_checkerboard` nested loop processes 1 pixel at a time — process 4-pixel SIMD chunks for 4× throughput. (P2)(B1)(e91442e)
- `export_as_image` rayon chunk too small — `par_chunks_mut(4)` spawns 4-byte tasks; increase to `par_chunks_mut(4096)`. (P3)(B1)(e91442e)

### Security

- undo snapshot decompression lacks size limit — `files.rs` uses `MAX_DECOMPRESSED_BYTES` but `undo.rs` does not, risking OOM on malicious saves. (P2)(B1)(e91442e)

### Architecture

- consolidate eraser variants — explore `ToolKind { Square, Circle }` + `Eraser(ToolKind)` layout (enum, not bool). (P2)(B4)(460008e)
- deduplicate stamp/brush fields in `ToolConfiguration` — share sub-struct for `sampling`/`tint_mode`. (P2)(B3)(460008e)
- line tool — click-drag straight line; preview during drag; shift-constrain to 45°. (P2)(B2)(460008e)
- add `Canvas` resize guard for undo records — version-stamp run segments against canvas dimensions so undo/redo are no-ops after resize. (P3)(B1)(460008e)
- `draw_bucket_fill` discards `before_pixels` at return — `before_pixels: Vec::new()` should be the populated variable; causes corrupted undo on non-uniform backgrounds. (P0)(B1)(e91442e)
- `zstd::encode_all` in `UndoRecord::compress_before`/`maybe_snapshot` panics on compression failure — return `Result` instead of `expect()`. (P2)(B1)(e91442e)
- drag accumulator `MAX_DRAG_FRAMES` split creates adjacent undo records for one gesture — merge both records during `finalize_drag_accumulator` when previous record matches the same drag. (P2)(B1)(e91442e)

### UX

### Documentation

- **(none at the moment)**

## Implementing

- None at the moment
