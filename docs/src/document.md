# document

## `struct Document`

Central state wrapper that owns a [`Canvas`], tracks the active layer index and
save path, and coordinates GPU texture uploads.

### Fields

| Field                       | Type     | Purpose                                             |
| --------------------------- | -------- | --------------------------------------------------- |
| `canvas`                    | `Canvas` | The backing canvas (layers, pixel data, dimensions) |
| `savefile_path`             | `String` | Filesystem path for the last save/load operation    |
| `current_layer`             | `usize`  | Index into `canvas.pixels` for the active layer     |
| `dirty_since_last_autosave` | `bool`   | Whether unsaved changes exist                       |

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

| Parameter | Type     | Description                |
| --------- | -------- | -------------------------- |
| `canvas`  | `Canvas` | The backing canvas to wrap |

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

| Parameter | Type               | Description                      |
| --------- | ------------------ | -------------------------------- |
| `canvas`  | `Canvas`           | New canvas to adopt              |
| `undo`    | `&mut UndoHistory` | Undo history to clear and resize |

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

| Parameter | Type                            | Description                                                            |
| --------- | ------------------------------- | ---------------------------------------------------------------------- |
| `queue`   | `&wgpu::Queue`                  | Queue for submitting write commands to the GPU                         |
| `texture` | `&wgpu::Texture`                | Destination GPU texture                                                |
| `dirty`   | `&Option<(u32, u32, u32, u32)>` | `Some((x, y, w, h))` for partial upload, `None` for full canvas upload |

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

---

## `Document::render_to_texture(ui)`

Blends all layers into `output_rgba`, then uploads the result to an egui texture
for display. This is the fallback path for the Glow backend, where wgpu
texture writes are not available. Always uploads the full texture — egui's
texture API does not support partial updates.

### Signature

```rust
pub fn render_to_texture(&mut self, ui: &egui::Ui)
```

### Parameters

| Parameter | Type        | Description                                  |
| --------- | ----------- | -------------------------------------------- |
| `ui`      | `&egui::Ui` | Egui UI handle used to access `load_texture` |

### Behaviour

1. Calls `blend_to_output()` to ensure `output_rgba` is up to date.
2. Constructs a `ColorImage` from the premultiplied RGBA buffer.
3. If `self.canvas.rendered_layers` already holds a texture, calls `set(...)` on
   it with the new image data.
4. Otherwise, creates a new named texture via `ui.ctx().load_texture(...)` and
   stores it in `self.canvas.rendered_layers`.

### Texture options

Uses `TextureOptions::LINEAR` for bilinear interpolation on the GPU.

---

## `Document::add_layer()`

Appends a new transparent layer to the canvas. The new layer is filled entirely
with `Color32::TRANSPARENT` and uses the same dimensions as the existing canvas.

### Signature

```rust
pub fn add_layer(&mut self)
```

### Behaviour

- Pushes a `Layer` containing `width × height` transparent pixels onto
  `self.canvas.pixels`.
- Sets `self.canvas.render_next_frame = true` so the compositor re-blends all
  layers on the next frame.
- Does **not** change `current_layer` — the newly added layer is appended at the
  end; the UI is responsible for selecting it if desired.

---

## `Document::delete_layer(index)`

Removes the layer at the given index and clamps `current_layer` to the new layer
count. Does **not** guard against deleting the last remaining layer — that
invariant is enforced by the UI layer (which should disable the delete button
when only one layer exists).

### Signature

```rust
pub fn delete_layer(&mut self, index: usize)
```

### Parameters

| Parameter | Type    | Description                  |
| --------- | ------- | ---------------------------- |
| `index`   | `usize` | Index of the layer to remove |

### Behaviour

1. Removes the entry from `self.canvas.pixels`.
2. Adjusts `current_layer`:
   - If the removed layer was below the active layer, `current_layer` is
     decremented by 1 (via `saturating_sub(1)`).
   - The result is clamped to `[0, layers.len() - 1]` with `min()`.
3. Sets `render_next_frame = true`.

---

## `Document::move_layer_up(index)`

Swaps the layer at `index` with the layer above it (`index - 1`) and updates
`current_layer` to follow the moved layer.

### Signature

```rust
pub fn move_layer_up(&mut self, index: usize)
```

### Parameters

| Parameter | Type    | Description                       |
| --------- | ------- | --------------------------------- |
| `index`   | `usize` | Index of the layer to move upward |

### Panics

Panics if `index == 0` — there is no layer above to swap with. The caller
(the UI) must ensure that `index > 0` before calling.

### Behaviour

- Performs `self.canvas.pixels.swap(index, index - 1)`.
- Sets `current_layer = index - 1` (the layer moves with the swap).
- Sets `render_next_frame = true`.

---

## `Document::move_layer_down(index)`

Swaps the layer at `index` with the layer below it (`index + 1`) and updates
`current_layer` to follow the moved layer.

### Signature

```rust
pub fn move_layer_down(&mut self, index: usize)
```

### Parameters

| Parameter | Type    | Description                         |
| --------- | ------- | ----------------------------------- |
| `index`   | `usize` | Index of the layer to move downward |

### Panics

Panics if `index >= pixels.len() - 1` — there is no layer below. The caller
(the UI) must ensure that `index < pixels.len() - 1` before calling.

### Behaviour

- Performs `self.canvas.pixels.swap(index, index + 1)`.
- Sets `current_layer = index + 1` (the layer moves with the swap).
- Sets `render_next_frame = true`.

---

## `Document::select_layer(index)`

Sets the current (active) layer index. Does **not** trigger a re-render — layer
selection only affects which layer receives future brush strokes.

### Signature

```rust
pub fn select_layer(&mut self, index: usize)
```

### Parameters

| Parameter | Type    | Description                  |
| --------- | ------- | ---------------------------- |
| `index`   | `usize` | Index of the layer to select |

### Behaviour

- Sets `self.current_layer = index`.
- No validation of the index is performed; the caller must ensure `index <
self.canvas.pixels.len()`.

## `Document::canvas_mut`

Returns a mutable reference to the underlying Canvas. Uses Arc::make_mut to clone-on-write, ensuring that if the Arc is shared (e.g. during async save) a fresh copy is created before mutation.

## `canvas field type Arc<Canvas>`

The canvas field is now Arc<Canvas> rather than Canvas directly. This enables clone-on-write semantics: during async save the canvas Arc is cloned cheaply (refcount bump), and the background thread gets a snapshot while the UI thread continues to mutate via make_mut.
