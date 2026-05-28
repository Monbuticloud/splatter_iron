# ADR 6: Dirty-Rect Incremental Rendering

- **Status:** Accepted
- **Date:** 2026-05-19
- **Commit:** `87ffef8`

## Context

Before this commit, every brush stroke triggered a full-canvas blend of all
layers (3M pixels for a 2000×1500 canvas), even when the stroke only modified
a few hundred pixels. This meant:

- Full SIMD+rayon blend every frame, saturating all CPU cores
- Full texture upload to GPU each frame
- No way to prioritise the repaint of only the changed region

Brush strokes produce small, localised changes. A typical drag stroke modifies
~1–10% of the canvas. Re-blending the untouched 90–99% is wasted work.

## Decision

Introduce a `DirtyRect` bounding-box tracker that records the axis-aligned
region modified by each tool operation, and a `blend_region` function that only
re-blends pixels within that rectangle.

### `DirtyRect` struct

```rust
pub struct DirtyRect {
    pub min_x: u32,  // inclusive
    pub min_y: u32,
    pub max_x: u32,  // inclusive
    pub max_y: u32,
}
```

- `extend(x, y)` expands the rect to include a point.
- `union(other)` merges two rects into their bounding box.
- `is_empty()` returns `true` when the rect covers no pixels.
- Every tool function (circle brush, square brush, bucket fill) tracks its
  footprint via `dirty_rect.extend()` during brush-line stepping.

### `blend_region` function

```rust
pub fn blend_region(
    layers: &[&[Color32]],
    output: &mut [u8],
    canvas_width: u32,
    min_x: u32, min_y: u32, max_x: u32, max_y: u32,
);
```

Iterates row-by-row through the dirty rectangle and reuses `blend_pixel_range`
(the same SIMD+rayon machinery) for each row segment. Sequential iteration is
used because dirty rects from brush strokes are small enough that parallel
overhead would dominate.

### Canvas flow

```
brush stroke → tool updates dirty_rect (union) → Canvas::dirty_rect = Some(rect)
    → next frame: blend_region if Some, blend_layers if None
    → GPU upload only the dirty sub-region (via write_texture offset)
    → dirty_rect = None
```

`None` means "full re-blend needed" — set on layer reorder, canvas resize,
import, or any operation where the dirty region is unknown.

## Consequences

- **Positive:** ~5–10× reduction in per-frame blend work for typical strokes.
- **Positive:** GPU upload is reduced from full-canvas (12 MB for 2000×1500)
  to dirty-rect (usually a few KB) via `wgpu::Queue::write_texture` with
  `TexelCopyBufferLayout` offset.
- **Positive:** The `DirtyRect` abstraction is simple and testable (8 unit
  tests cover extend, union, empty, width, height).
- **Negative:** Every tool must be aware of `DirtyRect` and call `extend()` /
  `union()` — forgetting to update `dirty_rect` causes visual artefacts (stale
  pixels). This was a source of bugs in the initial implementation.
- **Negative:** The `blend_region` row-by-row loop loses the full-canvas
  SIMD alignment advantage; each row segment may be shorter than 4 pixels,
  falling back to scalar blend.
- **Negative:** `DirtyRect` union is a coarse approximation — two distant
  stroke endpoints create a large rect that covers many unchanged pixels.
  A dirty-region list would be more accurate but more complex.
