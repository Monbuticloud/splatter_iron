# square_brush

Rectangular fill brush. Provides single-stamp `draw_square` and line-stroke
`draw_square_line` variants with visited-stamp deduplication.

## Internal: `fill_square_impl`

```rust
fn fill_square_impl(
    pixels: &mut [Color32],
    width: usize,
    start_x: u32,
    end_x: u32,
    start_y: u32,
    end_y: u32,
    color: Color32,
    alpha_overlay: bool,
)
```

Fills a rectangular region of a raw pixel slice without capturing undo data.
Used by `draw_square` after it has already captured before-pixels.

When `alpha_overlay` is true, each pixel is alpha-blended individually instead
of being bulk-filled with `.fill()`.

### Panics

Panics if `pixels` is not large enough to cover the rectangle at the given
`width`.

## Internal: `stamp_line_positions`

```rust
fn stamp_line_positions(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    brush_radius: u32,
    width: usize,
    height: u32,
    visited: &mut [u32],
    stamp: u32,
    dirty_rect: &mut DirtyRect,
)
```

Marks every pixel touched by a square-brush stroke line in the `visited` buffer
using the current `stamp` value.

### Algorithm

Uses Bresenham's line algorithm to step from `(start_x, start_y)` to
`(end_x, end_y)`. At each step, stamps a square brush footprint
`[cx − r, cx + r] × [cy − r, cy + r]` into the visited buffer.

Bounds are clamped to canvas dimensions. The `dirty_rect` is extended to cover
the full bounding box of all stamped positions.

### Panics

Panics if `visited` is shorter than `width * height`.

## `fn draw_square`

```rust
pub fn draw_square(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    canvas: &mut Canvas,
    color: egui::Color32,
    layer: usize,
    alpha_overlay: bool,
) -> UndoRecord
```

Draws a filled rectangle on a canvas layer, returns an undo record.

### Algorithm

1. Clamp all four coordinates to canvas dimensions.
2. If `start_x >= end_x` or `start_y >= end_y`, return an empty undo record.
3. Iterate over each row in `[start_y, end_y)`, capturing before-pixel data
   for the horizontal span `[start_x, end_x)`.
4. Fill the rectangle via `fill_square_impl`.
5. Union the bounding box into `canvas.dirty_rect`.

### Parameters

| Parameter | Type | Purpose |
|---|---|---|
| `start_x` | `u32` | Left column (inclusive). Clamped to `[0, canvas.width]`. |
| `start_y` | `u32` | Top row (inclusive). Clamped to `[0, canvas.height]`. |
| `end_x` | `u32` | Right column (exclusive). Clamped to `[0, canvas.width]`. |
| `end_y` | `u32` | Bottom row (exclusive). Clamped to `[0, canvas.height]`. |
| `canvas` | `&mut Canvas` | Canvas whose pixels are modified. |
| `color` | `Color32` | Fill colour (premultiplied-alpha). |
| `layer` | `usize` | Target layer index. |
| `alpha_overlay` | `bool` | Alpha-blend instead of overwrite. |

### Returns

`UndoRecord::Run` with compressed before-pixel runs. Returns an empty record
when `start_x >= end_x` or `start_y >= end_y` (degenerate rectangle).

### Panics

Panics if `layer >= canvas.pixels.len()`.

## `fn draw_square_line`

```rust
pub fn draw_square_line(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    brush_radius: u32,
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

Draws a square-brush stroke between two points, returns an undo record.

### Algorithm

1. Call `stamp_line_positions` to mark every covered pixel in the `visited`
   buffer with the current `stamp` value.
2. Iterate over the bounding box of the stroke. For each pixel whose `visited`
   matches `stamp` (and whose `drag_processed` does **not** match
   `drag_stamp_value` when alpha-blending), capture before-pixel data and write
   the new colour.
3. When `alpha_overlay` is true, the `drag_processed` buffer prevents blending
   the same pixel twice within a single drag motion.
4. Union the stroke's bounding box into `canvas.dirty_rect`.

### Parameters

| Parameter | Type | Purpose |
|---|---|---|
| `start_x` | `u32` | Start column |
| `start_y` | `u32` | Start row |
| `end_x` | `u32` | End column |
| `end_y` | `u32` | End row |
| `brush_radius` | `u32` | Brush radius in pixels |
| `canvas` | `&mut Canvas` | Canvas whose pixels are modified |
| `color` | `Color32` | Stroke colour (premultiplied-alpha) |
| `layer` | `usize` | Target layer index |
| `visited` | `&mut [u32]` | Stamp buffer for per-stroke deduplication |
| `stamp` | `u32` | Current stamp value (caller manages via `UndoHistory::next_stamp`) |
| `alpha_overlay` | `bool` | Alpha-blend instead of overwrite |
| `drag_processed` | `&mut [u32]` | Per-drag deduplication buffer (prevents double-blend) |
| `drag_stamp_value` | `u32` | Current drag stamp value |

### Returns

`UndoRecord::Run` with compressed before-pixel runs for every modified span.

### Panics

Panics if `layer >= canvas.pixels.len()`.

### Deduplication scheme

Identical to the scheme used in `circle_brush::draw_circle_line`:

- **`visited`**: Per-stroke stamp. Marks all pixels this stroke line covers.
- **`drag_processed`**: Accumulated across a drag motion. Prevents double-blending
  overlapping brush positions when `alpha_overlay` is true.
