# TODO

## Priority levels

- [P0] - Critical
- [P1] - High
- [P2] - Medium
- [P3] - Minor
- [P4] - Trivial
- [P5] - Nice to have

## Change level

- [B0] - No impact
- [B1] - Internal only
- [B2] - Compatible change
- [B3] - Deprecated interface
- [B4] - Breaking interface change
- [B5] - Architectural overhaul

## Format

(Change)\[PX\]\[BX\]\[Since what commit (use hash)\]

## Ideas

### Performance

- `export_as_image` pixel-by-pixel loop — `src/files.rs:247` → use `bytemuck` cast + parallel iteration. [P1][B1][aef7235]
- `compress_run` allocates `Vec<Color32>` before RLE check — `src/undo.rs:52` → defer allocation via iterator/callback. [P2][B1][aef7235]
- bucket fill lacks visited-stamp dedup — `src/tools/bucket_fill.rs:34` → reuse `apply_visited_runs`. [P2][B2][aef7235]
- `stamp_circle_positions` uses `f64::sqrt` per row — `src/tools/circle_brush.rs:312` → integer midpoint-circle increment. [P2][B1][aef7235]
- grid overlay redraws all lines every frame — `src/ui/center.rs:120-148` → cache as shape or vertex buffer. [P3][B1][aef7235]

### Architecture

- split `FileIO` — `src/file_io.rs:99` manages dialog/save/export/load/import/stamp/brush → `DialogManager`, `SaveManager`, `ExportManager`, `ImportManager`. [P1][B5][aef7235]
- `apply_stroke` duplicates `BrushStrokeParams` construction — `src/ui/center.rs:509-751` → extract builder. [P1][B1][aef7235]
- consolidate eraser variants — explore `ToolKind { Square, Circle }` + `Eraser(ToolKind)` layout (enum, not bool). [P2][B4][aef7235]
- deduplicate stamp/brush fields in `ToolConfiguration` — share sub-struct for `sampling`/`tint_mode`. [P2][B3][aef7235]
- layer blend modes — add Multiply, Screen, Overlay etc. (currently only alpha-overlay + opaque). [P2][B2][aef7235]
- selection tools — rectangular/lasso/magic-wand. [P2][B2][aef7235]

### UX

- keyboard shortcut system with help dialog — top-bar "Keyboard Shortcuts" / `?` button. [P2][B2][aef7235]
- pressure sensitivity — `pressure: f32` in `BrushStrokeParams` (per ADR-0018). [P3][B2][aef7235]

### Documentation

- README is sparse — add build instructions, feature list, screenshot, contribution guide. [P3][B0][aef7235]

## Accepted

-

## Denied

-

## Implementing

-

## Implemented

-
