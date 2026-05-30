# TODO

## Ideas

### Performance

- **`export_as_image` pixel-by-pixel** — `src/files.rs:247` loops per-pixel converting premultiplied→straight for all 13 formats. Use `bytemuck` cast + parallel iteration. ICO resize + JPEG blend also add overhead. No tests exist. [P1]
- **`compress_run` always allocates `Vec<Color32>` before checking** — `src/undo.rs:52` forces heap allocation even when run compresses to `All(Color32)`. Use iterator or callback to defer allocation. [P2]
- **Bucket fill lacks visited-stamp dedup** — `src/tools/bucket_fill.rs:34` doesn't use visited stamp. On large uniform regions the scanline stack can push massive `RunSegment`s. Reuse `apply_visited_runs` from `brush_common`. [P2]
- **`stamp_circle_positions` uses `f64::sqrt` per row** — `src/tools/circle_brush.rs:312` calls `sqrt()` inside Bresenham loop. Use integer midpoint-circle increment instead. [P2]
- **Grid overlay redraws all lines every frame** — `src/ui/center.rs:120-148` paints each grid line individually. For dense grids this is hundreds of shapes per frame. Cache as a shape or use vertex buffer. [P3]

### Architecture

- **`FileIO` has too many responsibilities** — `src/file_io.rs:99` manages dialog dispatching, save/load/import/export async ops, stamp loading, brush parsing, and export strategy. Split into `DialogManager`, `SaveManager`, `ExportManager`, `ImportManager`. [P1]
- **`apply_stroke` duplicates `BrushStrokeParams` construction** — `src/ui/center.rs:509-751` 200+ lines of near-identical struct building across 4 tool branches. Extract a builder or helper fn. [P1]
- **Reconsider eraser tool variants** — `CurrentTool::SquareEraser`/`CircleEraser` double variant count vs `Square`/`Circle`. Keep enum approach (not bool) but explore consolidated enum layout with `ToolKind { Square, Circle }` and `Eraser(ToolKind)`. [P2]
- **`ToolConfiguration` duplicates stamp/brush fields** — `stamp_sampling`/`brush_sampling`, `stamp_tint_mode`/`brush_tint_mode` are identical types. Use shared sub-struct. [P2]
- **No blend modes** — Only alpha-overlay and opaque replace. Add Multiply, Screen, Overlay, etc. per ADR-0003 compositing approach. [P2]
- **No selection tools** — No rectangular/lasso/magic-wand selection. All paint is freehand on full layer. [P2]

### UX

- **No keyboard shortcut system** — Add configurable keybinds with a help dialog accessible from a top-bar button ("Keyboard Shortcuts" or `?`). [P2]
- **No pressure sensitivity** — ADR-0018 mentions `pressure: f32` planned for `BrushStrokeParams`. Needs platform pen-event plumbing. [P3]

### Documentation

- **README is very sparse** — 3 bullet points and a copyright notice. Add build instructions, feature list, screenshot, contribution guide. [P3]

## Accepted

-

## Denied

-

## Implementing

-

## Implemented

-
