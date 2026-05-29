# bucket_fill

Scanline flood-fill tool. Replaces a contiguous region of same-coloured pixels
with a new colour, starting from a user-supplied seed point.

## `fn draw_bucket_fill`

```rust
pub fn draw_bucket_fill(
    seed_x: u32,
    seed_y: u32,
    canvas: &mut Canvas,
    color: Color32,
    layer: usize,
    alpha_overlay: bool,
) -> UndoRecord
```

Fills a contiguous region of matching colour starting from `(seed_x, seed_y)`.

### Algorithm

The implementation is a classic scanline flood-fill:

1. Read the colour at the seed point — this is the **target colour**.
2. If the seed already equals `color`, return an empty `UndoRecord` (no-op).
3. For each seed popped from a stack, scan left and right to find the
   contiguous horizontal span of target-colour pixels.
4. Record the before-pixel data for that span (for undo).
5. Write the new colour (or alpha-blend if `alpha_overlay` is true).
6. For the rows immediately above and below, scan the same horizontal extent
   and push any new runs of target-colour pixels onto the stack.
7. Repeat until the stack is empty.

### Parameters

| Parameter       | Type          | Purpose                                                                      |
| --------------- | ------------- | ---------------------------------------------------------------------------- |
| `seed_x`        | `u32`         | Column of the starting fill point. Clamped to `[0, canvas.width - 1]`.       |
| `seed_y`        | `u32`         | Row of the starting fill point. Clamped to `[0, canvas.height - 1]`.         |
| `canvas`        | `&mut Canvas` | The canvas whose pixels will be modified.                                    |
| `color`         | `Color32`     | Fill colour in premultiplied-alpha format.                                   |
| `layer`         | `usize`       | Index of the target layer.                                                   |
| `alpha_overlay` | `bool`        | When true, alpha-blends `color` over existing pixels instead of overwriting. |

### Returns

An `UndoRecord::Run` containing compressed before-pixel data for every
modified span, or an empty record if the seed already matches the fill colour.

### Panics

Panics if `layer >= canvas.pixels.len()`.

### Edge cases

- If the seed is out of bounds, it is silently clamped to the nearest valid
  pixel before reading the target colour.
- If the entire canvas is already the target colour, the function returns
  immediately with an empty undo record — no pixels are touched.
- A zero-area canvas (width or height == 0) would produce an empty dirty rect
  and an empty run list.
