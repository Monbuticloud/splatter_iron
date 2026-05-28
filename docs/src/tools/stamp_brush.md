# stamp_brush

External-image stamp brush.  Provides `draw_stamp_line` for placing a
user-supplied image onto the canvas at a single click or along an
interpolated drag path.

## Internal: `stamp_at`

```rust
fn stamp_at(
    center_x: u32,
    center_y: u32,
    stamp_pixels: &[Color32],
    stamp_width: u32,
    stamp_height: u32,
    output_w: u32,
    output_h: u32,
    canvas_width: u32,
    canvas_height: u32,
    layer_pixels: &mut [Color32],
    color: Color32,
    alpha_overlay: bool,
    tinted: bool,
    drag_processed: &mut [u32],
    drag_stamp_value: u32,
    runs: &mut Vec<RunSegment>,
    dirty_rect: &mut DirtyRect,
)
```

Stamps the image once centred at `(center_x, center_y)` and collects
compressed per-row run segments into `runs`.

### Algorithm

1. Compute the unclamped output bounding rectangle
   `[cx − w/2, cx + w/2] × [cy − h/2, cy + h/2]`.
2. Clamp to canvas dimensions; if entirely off-screen, return immediately.
3. For each pixel in the visible region:
   a. Check the `drag_processed` buffer (alpha-overlay dedup) — if this pixel
      was already blended in the current drag, close the current run and skip.
   b. Map the output pixel back to source stamp coordinates via nearest-neighbour
      sampling, accounting for output scaling.
   c. If `tinted` is true, multiply each premultiplied RGBA channel of the stamp
      pixel by the corresponding channel of `color` divided by 255.
   d. Capture the before-pixel, then write the (tinted) stamp pixel —
      either via `alpha_blend` or direct overwrite.
   e. When `alpha_overlay` is true, mark `drag_processed` so the pixel is not
      double-blended on a subsequent overlapping stamp in the same drag.

## `fn draw_stamp_line`

```rust
pub fn draw_stamp_line(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    stamp_pixels: &[Color32],
    stamp_width: u32,
    stamp_height: u32,
    radius: u32,
    canvas: &mut Canvas,
    color: Color32,
    layer: usize,
    alpha_overlay: bool,
    tinted: bool,
    drag_processed: &mut [u32],
    drag_stamp_value: u32,
) -> UndoRecord
```

Draws one or more stamp instances between two points on a canvas layer.

### Algorithm

1. Compute output dimensions: `output_w = radius.max(1)`,
   `output_h = (stamp_height * output_w / stamp_width).round().max(1)`.
   This preserves the stamp's aspect ratio.
2. Compute step interval = `output_w / 2` (minimum 1) for drag interpolation.
3. If `start == end` (single click), place one stamp.
4. Otherwise, compute the Euclidean distance between start and end; place
   stamps at `step`-sized intervals along the line via linear interpolation.
5. Each individual stamp is delegated to `stamp_at` which captures before-pixels
   and writes tinted/overlaid stamp pixels.
6. Union the accumulated dirty rect into `canvas.dirty_rect`.

### Scaling

The `radius` parameter controls the output stamp width on the canvas in pixels.
Height is scaled proportionally.  Nearest-neighbour sampling is used for
maximum performance and pixel-art clarity.

### Parameters

| Parameter | Type | Purpose |
|---|---|---|
| `start_x` | `u32` | Start column |
| `start_y` | `u32` | Start row |
| `end_x` | `u32` | End column |
| `end_y` | `u32` | End row |
| `stamp_pixels` | `&[Color32]` | Premultiplied stamp image pixels (row-major) |
| `stamp_width` | `u32` | Native width of the stamp image |
| `stamp_height` | `u32` | Native height of the stamp image |
| `radius` | `u32` | Output stamp width in canvas pixels |
| `canvas` | `&mut Canvas` | Canvas whose pixels are modified |
| `color` | `Color32` | Tool colour (premultiplied); used for tinting |
| `layer` | `usize` | Target layer index |
| `alpha_overlay` | `bool` | Alpha-blend instead of overwrite |
| `tinted` | `bool` | Multiply stamp pixels by `color` |
| `drag_processed` | `&mut [u32]` | Per-drag deduplication buffer |
| `drag_stamp_value` | `u32` | Current drag stamp value |

### Returns

`UndoRecord::Run` with compressed before-pixel runs for every modified span.

### Panics

Panics if `layer >= canvas.pixels.len()`.
