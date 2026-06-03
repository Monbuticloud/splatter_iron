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

- adaptive render quality — reduce blend resolution when zoomed far out. (P3)(B1)(460008e)
- avoid full layer pixel clone in `AddLayer` undo record — store placeholder that recreates transparent pixels on undo instead of cloning entire `Layer`. (P2)(B1)(460008e)
- refactor duplicated `read_canvas`/`read_canvas_xz` and `write_canvas`/`write_canvas_xz` into generic functions accepting a decoder/encoder factory. (P3)(B1)(460008e)

### Security

### Architecture

- consolidate eraser variants — explore `ToolKind { Square, Circle }` + `Eraser(ToolKind)` layout (enum, not bool). (P2)(B4)(460008e)
- deduplicate stamp/brush fields in `ToolConfiguration` — share sub-struct for `sampling`/`tint_mode`. (P2)(B3)(460008e)
- layer blend modes — add Multiply, Screen, Overlay etc. (currently only alpha-overlay + opaque). (P2)(B2)(460008e)
- selection tools — rectangular/lasso/magic-wand. (P2)(B2)(460008e)
- layer locking — add `alpha_lock` / `full_lock` fields to `Layer`, wire into blend and brush-apply. (P2)(B2)(460008e)
- canvas rotation/flip — store rotation/filp in `UIState`; apply view transform in `center.rs`. (P2)(B2)(460008e)
- line tool — click-drag straight line; preview during drag; shift-constrain to 45°. (P2)(B2)(460008e)
- rectangle/ellipse shape tools — unfilled/stroked shapes with configurable border width. (P3)(B2)(460008e)
- add `Canvas` resize guard for undo records — version-stamp run segments against canvas dimensions so undo/redo are no-ops after resize. (P3)(B1)(460008e)
- create ADR-0025: Error Handling Strategy — document panic-vs-Result philosophy with ADR template. (P3)(B0)(460008e)

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

### Standards & Cleanup

## Accepted

### Performance



### Security

- **(none at the moment)**

### Architecture



### UX



### Documentation

- **(none at the moment)**

## Implementing

- None at the moment
