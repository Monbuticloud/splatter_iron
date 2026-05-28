# canvas

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
