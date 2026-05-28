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

## `fn compress_run(pixels)`

`compress_run` implements run-length encoding for undo storage. It takes a contiguous run of before-stroke pixel colors and decides whether to store them compactly (as a single `Color32` via [`BeforePixels::All`]) or as the full vector ([`BeforePixels::Many`]).

The compression decision uses a threshold constant `RLE_SHORT_RUN_THRESHOLD = 8`. Runs shorter than 8 pixels always store the full vector because the allocation overhead of a `Vec` (3 words: pointer, length, capacity) exceeds the savings for very short spans.

### Parameters

| Parameter | Type | Purpose |
|-----------|------|---------|
| `pixels` | `Vec<Color32>` | Contiguous run of before-pixel colors to compress |

### Returns

`(BeforePixels, u32)` — a tuple of the compressed pixel data and the run length. The run length is always `pixels.len()` cast to `u32`.

### Decision matrix

| Condition | Result |
|-----------|--------|
| Run length < 8 | `Many(pixels)` — vector stored as-is |
| All pixels equal, length ≥ 8 | `All(color)` — single color |
| Not all equal, length ≥ 8 | `Many(pixels)` — vector must be stored |

### Performance

`compress_run` scans the full run to check uniformity, which is O(N). For uniform runs this is unavoidable — we must verify that all pixels match. For non-uniform runs the same scan is needed to determine that compression is inapplicable, so worst-case is identical to best-case.

## `enum UndoRecord`

`UndoRecord` encodes a single drawing stroke in a form sufficient to both undo (restore before-pixels) and redo (reapply after-pixels). It carries all metadata needed to locate the affected pixels and reconstruct both states.

Currently only the `Run` variant is defined. The enum is structured as a single-variant enum to allow future addition of alternative record types (e.g., full-layer snapshots, transform records) without breaking callers.

### Variant: `UndoRecord::Run`

The `Run` variant stores a stroke as a collection of compressed contiguous pixel runs. This representation is efficient for brush strokes, which typically touch many small disjoint regions per frame.

#### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `layer_index` | `usize` | Index of the layer that was modified, used to select the target layer in the `Canvas.pixels` array |
| `color_after` | `Color32` | Color applied by the stroke. For opaque strokes this is written directly; for alpha overlays it is blended with the existing pixel via [`alpha_blend`] |
| `runs` | `Vec<RunSegment>` | Compressed run-length segments preserving before-pixel data. Each segment covers a contiguous span |
| `is_alpha_overlay` | `bool` | Whether this stroke was drawn as an alpha overlay. Controls redo behavior: `true` blends `color_after` over existing pixels; `false` overwrites them outright |

### Invariants

- `layer_index` must be a valid index into `Canvas.pixels`. Violations cause a panic in `undo_apply`/`redo_apply`.
- The sum of all `runs[*].length` must account for every pixel the stroke touched.
- `runs` must be non-empty for a valid stroke record.

## `fn undo_apply(canvas, record)`

`undo_apply` restores canvas pixels to their state before the stroke was applied. It is the core undo primitive: given an [`UndoRecord`], it walks each [`RunSegment`] and writes the saved before-pixels back into the layer's pixel buffer.

### Algorithm

1. Destructure the record to extract `layer_index`, `runs`, and discard `color_after`/`is_alpha_overlay` (irrelevant for undo).
2. Index into `canvas.pixels[layer_index]` to get the target layer.
3. For each run segment:
   - If `before` is `BeforePixels::All(color)`, fill the `[start..start+length)` range with that single color using `fill()`, which becomes a fast `memset`.
   - If `before` is `BeforePixels::Many(pixels)`, copy the saved vector slice into the range using `copy_from_slice()`, which compiles to `memcpy`.

### Parameters

| Parameter | Type | Purpose |
|-----------|------|---------|
| `canvas` | `&mut Canvas` | The canvas whose layer pixels will be restored |
| `record` | `&UndoRecord` | The undo record containing before-pixel data |

### Performance

Both `fill()` and `copy_from_slice()` are SIMD-optimized by the standard library for `Color32` (4 bytes per element). For typical brush strokes covering thousands of pixels, undo completes in microseconds.

### Panics

Panics if any run segment's `start + length` exceeds the target layer's pixel buffer length. This indicates a corrupt or mismatched undo record — the record must have been built from a different canvas state or layer configuration.

## `fn redo_apply(canvas, record)`

`redo_apply` reapplies a previously undone stroke to the canvas. It is the inverse of [`undo_apply`]: where undo restores before-pixels, redo writes the stroke's `color_after` back to the affected pixels.

### Algorithm

1. Destructure the record to extract `layer_index`, `color_after`, `runs`, and `is_alpha_overlay`.
2. Index into `canvas.pixels[layer_index]` to get the target layer.
3. If `is_alpha_overlay` is `true`: for each run, iterate pixel-by-pixel and blend `color_after` over the current pixel using `alpha_blend(existing, color_after)`. Iteration is necessary because each existing pixel may have been modified by intermediate operations.
4. If `is_alpha_overlay` is `false`: for each run, fill the `[start..start+length)` range with `color_after` using `fill()` (fast `memset`).

### Alpha-overlay redo

The alpha-overlay path is deliberately slower (pixel-by-pixel iteration with function call per pixel) because it must handle the general case where intervening operations may have changed the base pixel values. This is an intentional correctness-vs-speed trade-off: opaque strokes can use a bulk fill, but alpha strokes must consult each current pixel.

### Parameters

| Parameter | Type | Purpose |
|-----------|------|---------|
| `canvas` | `&mut Canvas` | The canvas whose layer pixels will be re-stroked |
| `record` | `&UndoRecord` | The undo record containing after-pixel data |

### Panics

Panics if any run segment's `start + length` exceeds the target layer's pixel buffer length. Same invariant as [`undo_apply`]: the record must match the canvas's current layer configuration.
