# canvas

## `struct DirtyRect`

`DirtyRect` is an axis-aligned bounding box that tracks which region of the canvas has been modified since the last GPU texture upload. It enables partial texture updates: instead of re-uploading the full `output_rgba` buffer every frame, the renderer can upload only the pixels within the dirty rectangle.

A value of `None` in `Canvas::dirty_rect` signals that the entire composite needs to be recalculated from scratch (e.g. after a layer reorder or deletion).

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `min_x` | `u32` | Leftmost column (inclusive) |
| `min_y` | `u32` | Topmost row (inclusive) |
| `max_x` | `u32` | Rightmost column (inclusive) |
| `max_y` | `u32` | Bottommost row (inclusive) |

### Invariants

- An empty `DirtyRect` has `min_x > max_x || min_y > max_y` (inverted bounds). This is the state produced by `DirtyRect::empty()` and detected by `DirtyRect::is_empty()`.
- For a non-empty rect, `min_x <= max_x` and `min_y <= max_y` must hold.
- Coordinates are in pixel-space (not texture-space). `width() = max_x - min_x + 1` and `height() = max_y - min_y + 1` when non-empty.

## `DirtyRect::new`

```rust
pub const fn new(min_x: u32, min_y: u32, max_x: u32, max_y: u32) -> Self
```

Constructs a `DirtyRect` directly from explicit bounds.

### Parameters

| Parameter | Type | Purpose |
|-----------|------|---------|
| `min_x` | `u32` | Leftmost column (inclusive) |
| `min_y` | `u32` | Topmost row (inclusive) |
| `max_x` | `u32` | Rightmost column (inclusive) |
| `max_y` | `u32` | Bottommost row (inclusive) |

The caller is responsible for maintaining the invariant that `min_x <= max_x` and `min_y <= max_y` for a meaningful non-empty rect. Passing inverted bounds produces an empty rect indistinguishable from one created by `empty()`.

This function is `const` and can be used in static or constant contexts.

## `DirtyRect::empty`

```rust
pub const fn empty() -> Self
```

Creates an empty `DirtyRect` with inverted bounds (`min_x = MAX`, `min_y = MAX`, `max_x = 0`, `max_y = 0`). This sentinel state is the starting point before any pixel has been recorded.

The first call to `extend(point)` will overwrite these sentinel values with the point's coordinates, transforming the rect into a single-pixel region. Until then, `is_empty()` returns `true` and `width()`/`height()` return `0`.

This is the idiomatic constructor for incremental dirty tracking:
```rust
let mut dirty = DirtyRect::empty();
for pixel in stroke_pixels {
    dirty.extend(pixel.x, pixel.y);
}
```

## `DirtyRect::extend`

```rust
pub fn extend(&mut self, x: u32, y: u32)
```

Expands the dirty rectangle to include the pixel at `(x, y)`. If the rect is currently empty (inverted bounds), the first `extend` call sets all four bounds to `(x, y)`, producing a single-pixel rect. Every subsequent call expands the bounding box outward as needed.

### Parameters

| Parameter | Type | Purpose |
|-----------|------|---------|
| `x` | `u32` | Column index of the affected pixel |
| `y` | `u32` | Row index of the affected pixel |

### Performance

Runs in O(1) — a constant number of `min`/`max` comparisons. This is called once per modified pixel during brush stroke application and undo/redo, making its efficiency critical.

### Invariants

- After any number of `extend` calls, the rect is either still empty (if never called) or `min_x <= max_x && min_y <= max_y`.
- A pixel within the rect after `extend(x, y)` is guaranteed to be included in the final uploaded texture region, so no visual artifacts from partial updates.

## `DirtyRect::union`

```rust
pub fn union(&self, other: &Self) -> Self
```

Merges two dirty rectangles into one, producing the minimal axis-aligned bounding box that covers both input rects. This is used when consolidating dirty regions from multiple operations (e.g. merging the dirty rect from a brush stroke with the existing accumulated rect).

### Parameters

| Parameter | Type | Purpose |
|-----------|------|---------|
| `other` | `&Self` | The second rect to merge with `self` |

### Returns

A new `DirtyRect` whose bounds are the element-wise `min` of the two `min_*` fields and element-wise `max` of the two `max_*` fields.

### Invariants

- The result is empty iff both inputs are empty (since `u32::MAX` propagates through `min` and `0` through `max`).
- The result is always a superset (inclusive) of both input rects.

## `struct Layer`

`Layer` represents a single 2D raster layer within the canvas layer stack. Each layer stores its pixel data as a flat `Vec<Color32>` in premultiplied-alpha row-major order, indexed as `pixels[y * width + x]`.

The type derives `Default` (producing an empty pixel buffer), `Clone` for duplication during undo/redo snapshots, and `Serialize`/`Deserialize` for persistence to `.splattercanvas` files.

Layers are composited bottom-to-top by [`blend_layers()`] — later layers overlay earlier ones using premultiplied-alpha blending. A document starts with one transparent layer; users add, delete, reorder, and select layers through `Document`'s layer-management API.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `pixels` | `Vec<Color32>` | Premultiplied-alpha RGBA pixels, row-major order |

### Invariants

- `pixels.len()` must equal `width * height` of the parent `Canvas`. This invariant is maintained by `Canvas`'s constructors and resize operations.
- Pixel colors are stored in premultiplied-alpha form: each channel has already been multiplied by the alpha value. This avoids dark fringing during blending and is the native format for the compositing engine in [`pixel`].
