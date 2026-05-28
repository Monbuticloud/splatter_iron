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

## `DirtyRect::is_empty`

```rust
pub const fn is_empty(&self) -> bool
```

Returns `true` if the rect covers no pixels. This occurs when either dimension is inverted: `min_x > max_x` or `min_y > max_y`.

An empty rect is the initial state before any pixels are recorded (as produced by `empty()`). All brush operations and undo/redo calls check `is_empty()` before initiating a texture upload to avoid zero-area uploads.

### Returns

`true` if the bounding box is degenerate (no pixels covered), `false` otherwise.

### Usage in rendering

```rust
if let Some(dirty) = &canvas.dirty_rect {
    if !dirty.is_empty() {
        // upload only the sub-region to the GPU
    }
}
```

## `DirtyRect::width`

```rust
pub const fn width(&self) -> u32
```

Returns the number of columns in the dirty rectangle, or `0` if the rect is empty.

The width is computed as `max_x - min_x + 1` when non-empty, which yields the inclusive count of pixel columns. An empty rect returns `0` without performing arithmetic on the sentinel bounds.

### Returns

The column count of the bounding box, or `0` for an empty rect.

## `DirtyRect::height`

```rust
pub const fn height(&self) -> u32
```

Returns the number of rows in the dirty rectangle, or `0` if the rect is empty.

The height is computed as `max_y - min_y + 1` when non-empty, yielding the inclusive count of pixel rows. An empty rect returns `0` without arithmetic on sentinel bounds.

### Returns

The row count of the bounding box, or `0` for an empty rect.

## `struct Canvas`

`Canvas` is the core raster data structure: an ordered stack of [`Layer`]s plus metadata for compositing, rendering, and dirty tracking. It owns the pixel data, the cached composite output buffer, and the GPU texture handle.

The type derives `Clone` for undo/redo snapshotting and `Serialize`/`Deserialize` for file persistence. Serde skips transient GPU/rendering state (`rendered_layers`, `output_rgba`, `dirty_rect`).

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `pixels` | `Vec<Layer>` | Layer stack, bottom (index 0) to top |
| `height` | `u32` | Canvas height in pixels |
| `width` | `u32` | Canvas width in pixels |
| `rendered_layers` | `Option<TextureHandle>` | Cached GPU texture handle for the blended composite |
| `output_rgba` | `Vec<u8>` | Premultiplied-alpha RGBA output buffer (width × height × 4 bytes) |
| `dirty_rect` | `Option<DirtyRect>` | Bounding box of pixels changed since last upload; `None` triggers full re-blend |
| `render_next_frame` | `bool` | Flag requesting full re-render on next frame |

### Invariants

- All layers in `pixels` have exactly `width * height` pixels.
- After construction, `width * height` does not overflow `usize` (enforced by checked arithmetic in the constructor).
- `output_rgba` is either empty (before first render) or has exactly `(width * height * 4)` bytes.
- When `dirty_rect` is `Some(rect)` and `!rect.is_empty()`, only the sub-region within the rect needs a texture upload; when `None`, the entire composite must be regenerated.

## `impl Default for Canvas`

```rust
fn default() -> Self
```

Creates a default canvas with the dimensions `2000 × 1500` pixels and a single fully-transparent layer. The constants `DEFAULT_WIDTH` and `DEFAULT_HEIGHT` are defined at module scope in `src/canvas.rs`.

The default canvas starts with:
- One transparent layer (all pixels set to `Color32::TRANSPARENT`).
- An empty `output_rgba` buffer (allocated lazily on first render).
- No GPU texture handle (`rendered_layers: None`).
- No pre-existing dirty region (`dirty_rect: None`).
- `render_next_frame: true` to trigger immediate initial compositing.

### Panics

Panics if `DEFAULT_WIDTH * DEFAULT_HEIGHT` overflows `usize`. For 2000 × 1500 this is impossible on any practical platform (3 million elements).

### Usage

The `Default` impl is the primary construction path used by `Document::new()` and by serde's `Default` for deserializing legacy files that omit canvas fields.

## `impl Canvas`

### `Canvas::new`

```rust
pub fn new(width: u32, height: u32) -> Self
```

Creates a new canvas with the specified dimensions and a single transparent layer. This is the parameterized constructor used when creating a canvas of non-default size (e.g. from a new-document dialog).

The constructor allocates `width × height` pixels for the initial layer, each set to `Color32::TRANSPARENT`. Transient GPU state (`rendered_layers`, `output_rgba`, `dirty_rect`) starts empty, and `render_next_frame` is set to `true`.

### Parameters

| Parameter | Type | Purpose |
|-----------|------|---------|
| `width` | `u32` | Canvas width in pixels |
| `height` | `u32` | Canvas height in pixels |

### Panics

Panics if `width as usize * height as usize` overflows `usize`. This is an invariant violation: the canvas cannot represent more than `usize::MAX` pixels. In practice, this only occurs with astronomically large dimensions (e.g. > 4 gigapixels on 64-bit platforms).

## `enum CurrentTool`

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CurrentTool { ... }
```

Identifies the drawing tool currently selected in the UI tool panel. The enum is stored in [`ToolConfig`] and matched exhaustively in the canvas interaction handler to dispatch the correct brush function.

Each variant selects a different drawing primitive or operation:

### Variants

| Variant | Behaviour |
|---------|-----------|
| `Square` | Fill axis-aligned rectangles. Dispatches to [`square_brush::fill_rect`] which writes `UndoRecord` entries for every pixel in the dragged rectangle. |
| `Circle` | Fill circles using the midpoint circle algorithm. Dispatches to [`circle_brush::fill_circle`] for span-based fill, producing an `UndoRecord`. |
| `SquareEraser` | Erase by dragging a rectangular region. Sets affected pixels to `Color32::TRANSPARENT` with a square mask. Dispatches to [`square_brush::fill_rect`] with the eraser color. |
| `CircleEraser` | Erase by dragging a circular region. Sets affected pixels to `Color32::TRANSPARENT` with a circular mask. Dispatches to [`circle_brush::fill_circle`] with the eraser color. |
| `BucketFill` | Flood-fill a contiguous region of similar color using a scanline algorithm. Dispatches to [`bucket_fill::flood_fill`]. |

### Matching

The `Canvas` rendering code and `Document` interaction handlers exhaustively match on `CurrentTool`:

```rust
match tool_config.current_tool {
    CurrentTool::Square | CurrentTool::SquareEraser => { /* square brush dispatch */ }
    CurrentTool::Circle | CurrentTool::CircleEraser => { /* circle brush dispatch */ }
    CurrentTool::BucketFill => { /* flood fill dispatch */ }
}
```

Eraser variants reuse the same brush primitives as their fill counterparts but write `Color32::TRANSPARENT` as the stroke color.

## `enum RenderState`

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RenderState { ... }
```

Controls the egui repaint cadence to balance responsiveness against power consumption. The render state machine has three levels, each reducing GPU work further when the user is not actively drawing.

### Variants

| Variant | Repaint behaviour | Use case |
|---------|-------------------|----------|
| `ActiveWake(Duration)` | Request repaint every frame (via `ctx.request_repaint_after(dur)`). The `Duration` parameter controls the throttle — typically set to `Duration::ZERO` for immediate repaint during active brush strokes. | User is dragging a brush, erasing, or flood-filling. Every frame matters for low-latency cursor feedback. |
| `IdleThrottled` | No continuous repaint requests. egui will still repaint on input events (mouse move, key press), but no periodic wake-up. | User has stopped drawing but the viewport is focused. Throttling saves battery/CPU without sacrificing responsiveness to the next interaction. |
| `UnfocusedFrozen` | All GPU work suspended. The viewport (egui window) is not focused. No repaint requests are issued regardless of input. | Viewport lost focus. Zero GPU work until the user refocuses the window. |

### State transitions

The state machine transitions are managed by the [`app`] module's frame handler, which inspects the current tool activity and viewport focus each frame:

```text
ActiveWake ──(no activity for N frames)──▶ IdleThrottled
IdleThrottled ──(mouse down / key press)──▶ ActiveWake
IdleThrottled ──(viewport unfocused)──────▶ UnfocusedFrozen
UnfocusedFrozen ──(viewport refocused)────▶ IdleThrottled
```

The `ActiveWake` variant's `Duration` parameter is set from [`ToolConfig::undo_redo_steps_multiplier`] — during fast drawing the throttle duration may be increased slightly to batch repaints and reduce texture upload contention.

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
