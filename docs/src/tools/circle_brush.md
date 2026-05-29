# circle_brush

Midpoint-circle span-fill brush. Provides single-stamp and line-stroke variants
with visited-stamp deduplication for the line strokes.

## Internal: `fill_circle_impl`

```rust
fn fill_circle_impl(
    pixels: &mut [Color32],
    width: usize,
    center_x: u32,
    center_y: u32,
    radius: u32,
    color: Color32,
    canvas_width: u32,
    canvas_height: u32,
    alpha_overlay: bool,
)
```

Fills a circular region of a raw pixel slice without capturing undo data.
Used by `draw_circle` after it has already captured before-pixels.

### Algorithm

Uses the midpoint circle algorithm transformed into span filling: for each
`delta_y` in `0..=radius`, compute `delta_x = sqrt(r² − dy²)` and fill the
horizontal span `[center_x − dx, center_x + dx]` on rows `center_y ± dy`.
The centre row is filled only once (the `delta_y = 0` case covers it).

Rows or columns that fall outside the canvas are silently skipped via
`saturating_sub` and `.min()` clamping.

### Panics

Panics if `pixels` is not large enough to cover the accessed spans.

## `fn draw_circle`

```rust
pub fn draw_circle(
    center_x: u32,
    center_y: u32,
    radius: u32,
    canvas: &mut Canvas,
    color: egui::Color32,
    layer: usize,
    alpha_overlay: bool,
) -> UndoRecord
```

Draws a filled circle on a canvas layer, returns an undo record.

### Algorithm

1. Clamp `(center_x, center_y)` to canvas dimensions.
2. For each `delta_y` in `0..=radius`, compute the horizontal span endpoints
   using the midpoint-circle formula and capture before-pixel data for every
   touched span in both the top and bottom halves (skipping centre-row dupe).
3. Fill the circle in-place via `fill_circle_impl`.
4. Union the bounding box `[center ± radius]` into `canvas.dirty_rect`.

### Parameters

| Parameter       | Type          | Purpose                                                  |
| --------------- | ------------- | -------------------------------------------------------- |
| `center_x`      | `u32`         | Column of circle centre. Clamped to `[0, canvas.width]`. |
| `center_y`      | `u32`         | Row of circle centre. Clamped to `[0, canvas.height]`.   |
| `radius`        | `u32`         | Circle radius in pixels.                                 |
| `canvas`        | `&mut Canvas` | Canvas whose pixels are modified.                        |
| `color`         | `Color32`     | Fill colour (premultiplied-alpha).                       |
| `layer`         | `usize`       | Target layer index.                                      |
| `alpha_overlay` | `bool`        | Alpha-blend instead of overwrite.                        |

### Returns

`UndoRecord::Run` with compressed before-pixel runs. Returns an empty record
when the circle would produce no visible pixels (radius of zero, or centre
clamped out of bounds).

### Panics

Panics if `layer >= canvas.pixels.len()`.

## Internal: `stamp_circle_positions`

```rust
fn stamp_circle_positions(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    geo_radius: u32,
    width: usize,
    height: u32,
    visited: &mut [u32],
    stamp: u32,
    dirty_rect: &mut DirtyRect,
)
```

Marks every pixel touched by a circle-brush stroke line in the `visited` buffer
using the current `stamp` value.

### Algorithm

Uses Bresenham's line algorithm to step from `(start_x, start_y)` to
`(end_x, end_y)`. At each step, stamps the entire circular brush footprint
(using the same midpoint-circle span logic). When `geo_radius == 0`, stamps
only the single-pixel Bresenham line.

Bounds are clamped to canvas dimensions, and `dirty_rect` is extended to
cover the full bounding box of all stamped positions.

### Panics

Panics if `visited` is shorter than `width * height`.

## `fn draw_circle_line`

```rust
pub fn draw_circle_line(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    geo_radius: u32,
    canvas: &mut Canvas,
    color: egui::Color32,
    layer: usize,
    visited: &mut [u32],
    stamp: u32,
    alpha_overlay: bool,
    drag_processed: &mut [u32],
    drag_stamp_value: u32,
) -> UndoRecord
```

Draws a circle-brush stroke between two points, returns an undo record.

### Algorithm

1. Call `stamp_circle_positions` to mark every covered pixel in the `visited`
   buffer with the current `stamp` value.
2. Iterate over the bounding box of the stroke. For each pixel whose `visited`
   value matches `stamp` (and whose `drag_processed` value does **not** match
   `drag_stamp_value` when alpha-blending), capture before-pixel data and write
   the new colour.
3. When `alpha_overlay` is true, the `drag_processed` buffer prevents blending
   the same pixel twice within a single drag motion (the accumulator pattern).
4. Union the stroke's bounding box into `canvas.dirty_rect`.

### Parameters

| Parameter          | Type          | Purpose                                                            |
| ------------------ | ------------- | ------------------------------------------------------------------ |
| `start_x`          | `u32`         | Start column                                                       |
| `start_y`          | `u32`         | Start row                                                          |
| `end_x`            | `u32`         | End column                                                         |
| `end_y`            | `u32`         | End row                                                            |
| `geo_radius`       | `u32`         | Brush radius in pixels                                             |
| `canvas`           | `&mut Canvas` | Canvas whose pixels are modified                                   |
| `color`            | `Color32`     | Stroke colour (premultiplied-alpha)                                |
| `layer`            | `usize`       | Target layer index                                                 |
| `visited`          | `&mut [u32]`  | Stamp buffer for per-stroke deduplication                          |
| `stamp`            | `u32`         | Current stamp value (caller manages via `UndoHistory::next_stamp`) |
| `alpha_overlay`    | `bool`        | Alpha-blend instead of overwrite                                   |
| `drag_processed`   | `&mut [u32]`  | Per-drag deduplication buffer (prevents double-blend)              |
| `drag_stamp_value` | `u32`         | Current drag stamp value                                           |

### Returns

`UndoRecord::Run` with compressed before-pixel runs for every modified span.

### Panics

Panics if `layer >= canvas.pixels.len()`.

### Deduplication scheme

The two-level deduplication (`visited` + `drag_processed`) prevents double-applying
alpha-blended strokes when the brush circles overlap on consecutive line segments
of the same drag motion:

- **`visited`**: Reset per stroke. Marks all pixels that this stroke line covers.
- **`drag_processed`**: Accumulated across a drag. Pixels already blended in a
  previous segment are skipped on subsequent segments when `alpha_overlay` is true.
