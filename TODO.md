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

- adaptive render quality — reduce blend resolution when zoomed far out. (P3)(B1)(5d89e87)

### Architecture

- split `FileIO` — (now `DialogManager` + `SaveManager` + `ExportManager` + `ImportManager`) further decouple or consolidate common save/load coordination in `app/mod.rs`. (P1)(B5)(5d89e87)
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

- dead code audit — `#[cfg(test)]` gate or remove 21 dead items in `asset_library`, `canvas`, `files`, `undo_history`. (P3)(B1)(5d89e87)
