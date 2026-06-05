# brush_params

Shared parameter bundle for brush-stroke line-drawing functions.

[`BrushStrokeParams`] groups the common parameters that every `draw_*_line`
function accepts (start/end coordinates, canvas, colour, layer, visited/drag
stamps). Tool-specific parameters (radius, sampling, tinting, spacing, stamp
pixels) remain as individual arguments alongside this struct.

## `struct BrushStrokeParams`

```rust
pub struct BrushStrokeParams<'a> {
    pub start_x: u32,
    pub start_y: u32,
    pub end_x: u32,
    pub end_y: u32,
    pub canvas: &'a mut Canvas,
    pub color: Color32,
    pub layer: usize,
    pub visited: &'a mut [u32],
    pub stamp: u32,
    pub alpha_overlay: bool,
    pub drag_processed: &'a mut [u32],
    pub drag_stamp_value: u32,
}
```

| Field              | Type          | Purpose                                              |
| ------------------ | ------------- | ---------------------------------------------------- |
| `start_x`          | `u32`         | Column of line start point                           |
| `start_y`          | `u32`         | Row of line start point                              |
| `end_x`            | `u32`         | Column of line end point                             |
| `end_y`            | `u32`         | Row of line end point                                |
| `canvas`           | `&mut Canvas` | The canvas whose pixels will be modified             |
| `color`            | `Color32`     | Stroke colour (premultiplied-alpha)                  |
| `layer`            | `usize`       | Index of the target layer                            |
| `visited`          | `&mut [u32]`  | Per-stroke stamp buffer for pixel deduplication      |
| `stamp`            | `u32`         | Current stroke-scoped stamp value                    |
| `alpha_overlay`    | `bool`        | Whether to alpha-blend instead of overwriting        |
| `drag_processed`   | `&mut [u32]`  | Per-drag-gesture dedup buffer for alpha blend frames |
| `drag_stamp_value` | `u32`         | Current drag-scoped stamp value                      |

### Usage

Every `draw_*_line` function in `src/tools/` takes this bundle plus
tool-specific arguments, reducing visible boilerplate and making
signatures easier to read.
