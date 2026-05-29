# custom_brush

Custom brush line drawing.  Provides `draw_custom_brush_line` for
stamping a brush tip onto the canvas at a single click or along an
interpolated drag path, with spacing derived from the brush's native
spacing percentage.

Delegates the per-stamp rendering to `stamp_brush::draw_stamp_line`,
reusing the same tinting, alpha-overlay blending, and scaling logic.

## `fn draw_custom_brush_line`

```rust
pub fn draw_custom_brush_line(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    tip_pixels: &[Color32],
    tip_width: u32,
    tip_height: u32,
    radius: u32,
    spacing_pct: u8,
    canvas: &mut Canvas,
    color: Color32,
    layer: usize,
    visited: &mut [u32],
    stamp: u32,
    alpha_overlay: bool,
    tinted: bool,
    sampling: StampSampling,
    drag_processed: &mut [u32],
    drag_stamp_value: u32,
) -> UndoRecord
```

### Algorithm

1. **Compute output dimensions**: `output_w = radius.max(1)`;
   `output_h = (tip_height * output_w / tip_width).round().max(1)`.
   This preserves the brush tip's aspect ratio.

2. **Compute step size**: `step = output_w * (spacing_pct / 100)`.
   The step is clamped to a minimum of 1 pixel to avoid division by zero
   or zero-step loops.

3. **Line interpolation**: If `start == end` (single click), one stamp is
   placed.  Otherwise the Euclidean distance is computed and divided by
   `step` to determine the number of stamps to place.  Each stamp is
   positioned at evenly-spaced intervals along the line.

4. **Per-stamp delegation**: Each individual stamp is drawn by calling
   `draw_stamp_line` with identical start and end coordinates (single
   stamp mode).  The resulting `UndoRecord::Run` segments are collected
   and merged into a single `UndoRecord`.

5. **Frame invalidation**: `canvas.render_next_frame = true` signals the
   renderer to redraw.

### Spacing

The `spacing_pct` parameter comes from the brush metadata (GBR spacing
field, ABR `spac` tag, or default 25 %).  A higher percentage means
fewer stamps per drag stroke, resulting in a more widely-spaced brush
impression.  Values typically range from 1–100.  The minimum step is
always 1 pixel even for spacing = 0.

### Parameters

| Parameter | Type | Purpose |
|---|---|---|
| `start_x` | `u32` | Start column (click 1) |
| `start_y` | `u32` | Start row |
| `end_x` | `u32` | End column (click 2 / drag position) |
| `end_y` | `u32` | End row |
| `tip_pixels` | `&[Color32]` | Premultiplied brush tip image (row-major) |
| `tip_width` | `u32` | Native width of the tip image |
| `tip_height` | `u32` | Native height of the tip image |
| `radius` | `u32` | Output stamp width in canvas pixels |
| `spacing_pct` | `u8` | Spacing percentage (0–100) from brush metadata |
| `canvas` | `&mut Canvas` | The canvas to draw on |
| `color` | `Color32` | Tool colour (premultiplied); used for tinting |
| `layer` | `usize` | Target layer index |
| `visited` | `&mut [u32]` | Per-stroke pixel dedup buffer |
| `stamp` | `u32` | Stroke-scoped stamp value for `visited` |
| `alpha_overlay` | `bool` | Alpha-blend instead of overwrite |
| `tinted` | `bool` | Multiply tip pixels by `color` |
| `sampling` | `StampSampling` | Nearest or bilinear pixel sampling |
| `drag_processed` | `&mut [u32]` | Per-drag dedup buffer (alpha overlay) |
| `drag_stamp_value` | `u32` | Current drag-scoped stamp value |

### Returns

`UndoRecord::Run` with merged run segments for all stamps placed during
the call.

### Panics

Panics if `layer >= canvas.pixels.len()`.
