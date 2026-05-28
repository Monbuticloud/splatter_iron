# document

## `struct Document`

Central state wrapper that owns a [`Canvas`], tracks the active layer index and
save path, and coordinates GPU texture uploads.

### Fields

| Field | Type | Purpose |
|---|---|---|
| `canvas` | `Canvas` | The backing canvas (layers, pixel data, dimensions) |
| `savefile_path` | `String` | Filesystem path for the last save/load operation |
| `current_layer` | `usize` | Index into `canvas.pixels` for the active layer |
| `dirty_since_last_autosave` | `bool` | Whether unsaved changes exist |

### Invariants

- `current_layer` must always be a valid index into `canvas.pixels`. Layer
  mutation methods (add, delete, move) adjust it to stay in range.
- `dirty_since_last_autosave` is set to `true` by the autosave loop when a
  stroke or layer change is detected, and reset to `false` after a successful
  autosave or explicit save.

---

## `Document::new(canvas)`

Constructs a `Document` wrapping the supplied `Canvas`.

Initialises the document with an empty save path, `current_layer = 0`, and
`dirty_since_last_autosave = false`.

### Signature

```rust
pub fn new(canvas: Canvas) -> Self
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `canvas` | `Canvas` | The backing canvas to wrap |

### Behaviour

- Stores the canvas as-is; no copy or clone is made.
- All other fields are set to their default initial values.

---

## `Document::replace_canvas(canvas, undo)`

Replaces the current canvas with a new one and resets the document to a clean
state. The undo history is cleared and resized to match the new canvas's pixel
count.

### Signature

```rust
pub fn replace_canvas(&mut self, canvas: Canvas, undo: &mut UndoHistory)
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `canvas` | `Canvas` | New canvas to adopt |
| `undo` | `&mut UndoHistory` | Undo history to clear and resize |

### Side effects

- `self.canvas` is replaced entirely.
- `self.savefile_path` is cleared to `""`.
- `self.dirty_since_last_autosave` is set to `false`.
- `undo.clear()` is called, discarding all saved undo/redo records.
- `undo.resize_visited(...)` reallocates the visited buffer to `width × height`
  of the new canvas.
- `self.canvas.render_next_frame` is set to `true` to force a re-render.

---

## `Document::blend_to_output()`

Blends all layers into the `output_rgba` buffer. When a `dirty_rect` is set,
only the pixels within that rectangle are recomputed (`blend_region`); otherwise
the entire canvas is blended from scratch (`blend_layers`).

### Signature

```rust
pub fn blend_to_output(&mut self) -> Option<(u32, u32, u32, u32)>
```

### Returns

- `Some((x, y, width, height))` — the bounding box of the region that was
  blended, usable as a partial upload hint.
- `None` — the dirty rectangle was empty; no blending was performed.

### Panics

Panics if the underlying blend operation encounters mismatched layer lengths or
an undersized output buffer. This signals a logic bug in the layer management
code.

### Side effects

- Resizes `output_rgba` to `width × height × 4` if the dimensions have changed.
- Sets `render_next_frame = false`.
- After blending, resets `dirty_rect` to `None`.

---

## `Document::upload_to_gpu(queue, texture, dirty)`

Uploads the blended `output_rgba` buffer (or a sub-region) to a wgpu GPU
texture. Designed for partial-rect uploads to avoid re-uploading the entire
canvas when only a small area changed.

### Signature

```rust
pub fn upload_to_gpu(
    &self,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    dirty: &Option<(u32, u32, u32, u32)>,
)
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `queue` | `&wgpu::Queue` | Queue for submitting write commands to the GPU |
| `texture` | `&wgpu::Texture` | Destination GPU texture |
| `dirty` | `&Option<(u32, u32, u32, u32)>` | `Some((x, y, w, h))` for partial upload, `None` for full canvas upload |

### Behaviour

- When `dirty` is `None`, the offset is `(0, 0)` and the extent is the full
  canvas dimensions.
- When `dirty` is `Some(...)` with zero width or height, the function returns
  immediately without issuing a GPU write.
- The `bytes_per_row` is always `canvas_width × 4` (the full row pitch),
  regardless of the dirty rect width. This matches the layout of `output_rgba`.

### Panics

Panics if `dirty` coordinates exceed the texture bounds or if the computed byte
offset falls outside `output_rgba`.
