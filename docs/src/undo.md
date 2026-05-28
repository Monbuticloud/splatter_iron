# undo

## `enum BeforePixels`

`BeforePixels` provides compressed storage for a run of pixels as they existed before a stroke was applied. Rather than always storing a full `Vec<Color32>`, runs whose pixels are all identical are stored as a single `Color32` value, saving memory and improving cache performance for the common case of drawing over a uniform background.

### Variants

| Variant | Data | When used |
|---------|------|-----------|
| `All(Color32)` | Single color value | Every pixel in the run had the same original color (run is long enough to benefit from compression) |
| `Many(Vec<Color32>)` | Full pixel vector | Pixels in the run had distinct colors, or the run was too short for RLE to be worthwhile |

### Memory trade-off

`All` stores 4 bytes (one `Color32`). `Many` stores `N * 4` bytes for the vector allocation plus heap overhead. The `compress_run` function uses a threshold of 8 pixels: runs shorter than 8 are never compressed because the vector overhead dominates for very short spans. For a 100-pixel uniform run, `All` saves 396 bytes versus storing the full vector.

## `struct RunSegment`

`RunSegment` describes a contiguous range of pixels within a layer's flat pixel array, along with their original color values before a stroke modified them. It is the atomic unit of undo data: when an undo record is applied, the original pixels in each segment are restored.

A stroke touching many disconnected regions of the canvas produces multiple `RunSegment` values within a single `UndoRecord::Run`. The runs are stored in the order they were visited during the stroke, which is also the order they are applied during undo/redo.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `start` | `u32` | Starting pixel index within the layer's flat `Vec<Color32>` (row-major order, `y * width + x`). Zero-based. |
| `length` | `u32` | Number of contiguous pixels in this run. Must be at least 1. |
| `before` | `BeforePixels` | Compressed storage of the pixel values before the stroke modified them. See [`BeforePixels`] for the compression scheme. |

### Invariants

- `start + length` must not exceed the layer's pixel buffer length. Violations cause a panic in [`undo_apply`].
- The number of color values in `before` (1 if `All`, `length` if `Many`) matches `length`. This is guaranteed by [`compress_run`] during record construction.
