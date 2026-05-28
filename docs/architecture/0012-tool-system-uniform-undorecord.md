# ADR 12: Tool System with Uniform UndoRecord Interface

- **Status:** Accepted
- **Date:** 2026-05-19
- **Commit:** `41aa053`

## Context

As the application grew beyond a single square brush, a pattern emerged: every
drawing tool (square, circle, eraser, bucket fill) needs to:

1. Modify pixels on a specific layer of the canvas
2. Track which pixels were modified (for `DirtyRect`)
3. Capture before-pixel data (for undo)
4. Return enough information to revert or redo the stroke

Initially, all brush functions lived in `canvas.rs` and used different calling
conventions — some took raw pixel indices, others took coordinate ranges, and
there was no shared undo type (early versions used `Stroke { pixels:
Vec<StrokePixel> }` directly).

## Decision

Extract all brush primitives into a `src/tools/` module hierarchy, each
exporting functions that:

1. Accept `&mut Canvas` as a parameter (not `&mut [Color32]` or raw indices)
2. Return `UndoRecord` (the unified undo type from ADR-0007)
3. Update `canvas.dirty_rect` via `union()` before returning

```rust
// src/tools/mod.rs
pub mod bucket_fill;
pub mod circle_brush;
pub mod square_brush;

// Uniform signature:
pub fn draw_square(..., canvas: &mut Canvas, ...) -> UndoRecord;
pub fn draw_circle(..., canvas: &mut Canvas, ...) -> UndoRecord;
pub fn draw_bucket_fill(..., canvas: &mut Canvas, ...) -> UndoRecord;
pub fn draw_square_line(..., canvas: &mut Canvas, ...) -> UndoRecord;
pub fn draw_circle_line(..., canvas: &mut Canvas, ...) -> UndoRecord;
```

Each function:
- Takes a `layer: usize` parameter specifying the target layer
- Takes `alpha_overlay: bool` to toggle between opaque fill and alpha blend
- Uses `compress_run()` to compress before-pixel data
- Updates `canvas.dirty_rect` with the bounding box of touched pixels

### `CurrentTool` enum

```rust
pub enum CurrentTool {
    Square,
    Circle,
    SquareEraser,    // uses TRANSPARENT color, no alpha overlay
    CircleEraser,    // uses TRANSPARENT color, no alpha overlay
    BucketFill,      // handled via click, not drag
}
```

Erasers are not separate tools — they are `Square`/`Circle` with
`Color32::TRANSPARENT` and `alpha_overlay = false`.

### Dispatch in `apply_stroke`

```rust
fn apply_stroke(&mut self, pixel_x: u32, pixel_y: u32) -> Option<UndoRecord> {
    match self.tool_configuration.current_tool {
        CurrentTool::Square | CurrentTool::SquareEraser => { ... }
        CurrentTool::Circle | CurrentTool::CircleEraser => { ... }
        CurrentTool::BucketFill => None, // handled via click
    }
}
```

## Consequences

- **Positive:** Adding a new tool requires only a new file in `src/tools/`
  implementing the `fn(..., &mut Canvas, ...) -> UndoRecord` signature — no
  changes to canvas.rs or the undo system.
- **Positive:** `UndoRecord` is the single contract between tools and the undo
  history — tools don't need to know about `UndoHistory`, `visited` buffers,
  or `DragAccumulator`.
- **Positive:** test modules mirror the tool modules (`tests::circle_brush`,
  `tests::square_brush`, `tests::bucket_fill`), enabling focused unit tests.
- **Negative:** Each tool function takes 7–10 parameters (start/end coordinates,
  radius, canvas, color, layer, overlay flag) — a "parameter object" pattern
  would reduce verbosity but hasn't been extracted yet.
- **Negative:** Line-drawing tools (square_line, circle_line) need access to
  `visited` and `drag_processed` buffers from `UndoHistory`, which breaks the
  clean separation — these buffers are passed as `&mut [u32]` parameters.
