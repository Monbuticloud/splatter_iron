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
