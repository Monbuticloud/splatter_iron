# ADR 19: Expanded Tool System

- **Status:** Accepted
- **Date:** 2026-06-01

## Context

ADR 11 introduced three tool modules (`bucket_fill`, `circle_brush`,
`square_brush`) and a 5-variant `CurrentTool` enum (Square, Circle,
SquareEraser, CircleEraser, BucketFill). The system has since grown
significantly:

- `CurrentTool` now has 9 variants — added Stamp, CustomBrush, Eyedropper,
  and Pan.
- `src/tools/` grew from 3 modules to 7: `brush_common`, `brush_parsers`,
  `bucket_fill`, `circle_brush`, `custom_brush`, `square_brush`,
  `stamp_brush`.
- ADR 11 noted that tool functions took 7–10 parameters each and that a
  "parameter object" pattern would reduce verbosity. ADR 18 (shared
  `BrushStrokeParams`) resolved this by bundling the 12 common parameters
  into a single struct.

## Decision

The tool system maintains the same architectural boundary established in
ADR 11 — each module exports functions that accept a canvas reference and
return an `UndoRecord` — but with these expansions:

### CurrentTool (9 variants)

Defined in `src/canvas.rs:346`:

```rust
pub enum CurrentTool {
    Square,         // solid rectangles by dragging
    Circle,         // solid circles by dragging
    SquareEraser,   // rectangular eraser
    CircleEraser,   // circular eraser
    BucketFill,     // flood-fill contiguous region
    Stamp,          // stamp external image onto canvas
    CustomBrush,    // draw using loaded brush tip
    Eyedropper,     // pick color from canvas
    Pan,            // pan canvas viewport by dragging
}
```

Erasers remain the same approach from ADR 11 — they are `Square`/`Circle`
with `Color32::TRANSPARENT` — but are now explicitly dispatched as separate
enum variants for UI clarity.

### Tool modules (7)

`src/tools/mod.rs` registers:

| Module | Role |
|---|---|
| `brush_common` | Shared visited-pixel run capture |
| `brush_parsers` | `.abr`, `.gbr`, `.brush` file format parsers |
| `bucket_fill` | Scanline flood-fill implementation |
| `circle_brush` | Midpoint-circle brush line drawing |
| `custom_brush` | Custom brush tip line drawing |
| `square_brush` | Rectangular brush line drawing |
| `stamp_brush` | External-image stamp tool |

### BrushStrokeParams integration

ADR 18's `BrushStrokeParams` bundles the 12 common parameters (start/end
coordinates, canvas, colour, layer, visited/drag buffers, alpha-overlay flag)
that every `draw_*_line` function needs. Tool-specific parameters (radius,
spacing, stamp pixels, sampling/tinting modes) remain as individual arguments
alongside the bundle.

## Consequences

- **Positive:** Adding a new tool requires one new variant in `CurrentTool`,
  one new file in `src/tools/`, and one match arm in the dispatch function —
  no changes to existing tool modules.
- **Positive:** `BrushStrokeParams` resolved the parameter-count concern from
  ADR 11 — signatures collapsed from 7–10 parameters to `(params, ...tool_specific)`.
- **Positive:** 9 tool variants cover all current drawing interactions
  (paint, erase, fill, stamp, custom brush, colour pick, pan).
- **Negative:** The `CurrentTool` enum and tool dispatch are in separate
  files (`canvas.rs` vs `app/mod.rs`), making it easy to forget updating one
  when adding a tool.
- **Negative:** `Pan` and `Eyedropper` are viewport/UI operations, not
  pixel-modifying tools — they share the `CurrentTool` enum with painting
  tools despite having no `UndoRecord` output.
