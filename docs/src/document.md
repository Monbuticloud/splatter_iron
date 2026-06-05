# document

## `struct Document`

Central state wrapper that owns a canvas (behind an `Arc<Canvas>` for
clone-on-write during async saves), tracks the active layer index and
save path, and coordinates GPU texture uploads.

### Fields

| Field                       | Type          | Purpose                                                                                    |
| --------------------------- | ------------- | ------------------------------------------------------------------------------------------ |
| `canvas`                    | `Arc<Canvas>` | The backing canvas behind an `Arc` for clone-on-write during async saves                   |
| `savefile_path`             | `String`      | Filesystem path for the last save/load operation                                           |
| `current_layer`             | `usize`       | Index into `canvas.pixels` for the active layer                                            |
| `dirty_since_last_autosave` | `bool`        | Whether unsaved changes exist                                                              |
| `save_state`                | `SaveState`   | Current save state — `Idle` or `InFlight` while an async save runs                         |
| `next_layer_number`         | `usize`       | Monotonically increasing counter for default layer names (unique across add/delete cycles) |

### Invariants

- `current_layer` must always be a valid index into `canvas.pixels`. Layer
  mutation methods (add, delete, move) adjust it to stay in range.
- `dirty_since_last_autosave` is set to `true` by the autosave loop when a
  stroke or layer change is detected, and reset to `false` after a successful
  autosave or explicit save.

---

## `Document::new(canvas)`

Constructs a `Document` wrapping the supplied `Canvas`.

Initialises the document with an empty save path, `current_layer = 0`,
`dirty_since_last_autosave = false`, and `next_layer_number` set to one
past the number of pre-existing layers.

### Signature

```rust
pub fn new(canvas: Canvas) -> Self
```

### Parameters

| Parameter | Type     | Description                |
| --------- | -------- | -------------------------- |
| `canvas`  | `Canvas` | The backing canvas to wrap |

### Behaviour

- Wraps the canvas in `Arc::new(canvas)` for clone-on-write semantics during async saves.
- All other fields are set to their default initial values.

---

## `Document::replace_canvas(canvas, undo)`

Replaces the current canvas with a new one and resets the document to a clean
state. The undo history is cleared and resized to match the new canvas's pixel
count. `next_layer_number` is set to `pixels.len() + 1` so subsequent default
layer names stay unique.

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
- `self.canvas_mut().dirty_rect.request_full_blend()` is called to force a full re-composite.

---

## `Document::blend_to_output()`

Blends all layers into the `output_rgba` buffer. Consumes dirty rects from
`DirtyRectList::take_all()`: when rects are present, only those regions are
recomputed via `blend_region`; when the list returns empty (full blend
requested), the entire canvas is blended from scratch via `blend_layers`.

### Signature

```rust
pub fn blend_to_output(&mut self) -> Option<DirtyRect>
```

### Returns

- `Some(DirtyRect)` — the bounding box of the region that was
  blended, usable as a partial upload hint.
- `None` — the dirty rectangle was empty; no blending was performed.

### Panics

Panics if the underlying blend operation encounters mismatched layer lengths or
an undersized output buffer. This signals a logic bug in the layer management
code.

### Side effects

- Resizes `output_rgba` to `width × height × 4` if the dimensions have changed.
- Calls `canvas_mut().dirty_rect.clear()` and marks `needs_full_blend = false` after blending.

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
    dirty: &Option<DirtyRect>,
)
```

### Parameters

| Parameter | Type                 | Description                                                         |
| --------- | -------------------- | ------------------------------------------------------------------- |
| `queue`   | `&wgpu::Queue`       | Queue for submitting write commands to the GPU                      |
| `texture` | `&wgpu::Texture`     | Destination GPU texture                                             |
| `dirty`   | `&Option<DirtyRect>` | `Some(DirtyRect)` for partial upload, `None` for full canvas upload |

### Behaviour

- When `dirty` is `None`, the offset is `(0, 0)` and the extent is the full
  canvas dimensions.
- When `dirty` is `Some(DirtyRect)` with zero width or height, the function returns
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
3. If `self.canvas_mut().rendered_layers` already holds a texture, calls `set(...)` on
   it with the new image data.
4. Otherwise, creates a new named texture via `ui.ctx().load_texture(...)` and
   stores it in `self.canvas_mut().rendered_layers`.

### Texture options

Uses `TextureOptions::LINEAR` for bilinear interpolation on the GPU.

---

## `Document::add_layer()`

Appends a new transparent layer to the canvas and selects it. The new layer is
filled entirely with `Color32::TRANSPARENT` and uses the same dimensions as the
existing canvas.

### Signature

```rust
pub fn add_layer(&mut self, undo: &mut UndoHistory)
```

### Behaviour

- Pushes a `Layer` containing `width × height` transparent pixels onto
  `self.canvas_mut().pixels`.
- Sets the layer's default name to `"Layer {n}"` where `n` is the current
  `next_layer_number` (incremented after each call so names remain unique
  across add/delete cycles).
- Sets `self.current_layer` to the index of the newly added layer so it becomes
  the active drawing target.
- Calls `self.canvas_mut().dirty_rect.request_full_blend()` so the compositor re-blends all
  layers on the next frame.
- Pushes `UndoRecord::AddLayer` onto the undo stack for revertability.

---

## `Document::delete_layer(index)`

Removes the layer at the given index and adjusts `current_layer` to account for
the removed entry. Does **not** guard against deleting the last remaining
layer — that invariant is enforced by the UI layer (which should disable the
delete button when only one layer exists).

### Signature

```rust
pub fn delete_layer(&mut self, index: usize, undo: &mut UndoHistory)
```

### Parameters

| Parameter | Type               | Description                      |
| --------- | ------------------ | -------------------------------- |
| `index`   | `usize`            | Index of the layer to remove     |
| `undo`    | `&mut UndoHistory` | Undo history for the undo record |

### Behaviour

1. Removes the entry via `self.canvas_mut().pixels.remove(index)`.
2. Adjusts `current_layer`:
   - If `index <= current_layer` (deleted layer at or below the active one),
     `current_layer` is decremented by 1 (via `saturating_sub(1)`).
   - If `index > current_layer` (deleted layer above the active one),
     `current_layer` is unchanged.
   - The result is clamped to `[0, layers.len() - 1]` with `min()` so it never
     exceeds the new layer count.
3. Pushes `UndoRecord::DeleteLayer` onto the undo stack.
4. Calls `self.canvas_mut().dirty_rect.request_full_blend()`.

---

## `Document::move_layer_up(index)`

Swaps the layer at `index` with the layer above it (`index - 1`) and updates
`current_layer` to follow the moved layer.

### Signature

```rust
pub fn move_layer_up(&mut self, index: usize, undo: &mut UndoHistory)
```

### Parameters

| Parameter | Type               | Description                       |
| --------- | ------------------ | --------------------------------- |
| `index`   | `usize`            | Index of the layer to move upward |
| `undo`    | `&mut UndoHistory` | Undo history for the undo record  |

### Panics

Panics if `index == 0` — there is no layer above to swap with. The caller
(the UI) must ensure that `index > 0` before calling.

### Behaviour

- Gets a mutable canvas via `self.canvas_mut()` and swaps layers with
  `canvas.pixels.swap(index, index - 1)`.
- Sets `current_layer = index - 1` (the layer moves with the swap).
- Pushes `UndoRecord::MoveLayer` onto the undo stack.
- Calls `canvas_mut().dirty_rect.request_full_blend()`.

---

## `Document::move_layer_down(index)`

Swaps the layer at `index` with the layer below it (`index + 1`) and updates
`current_layer` to follow the moved layer.

### Signature

```rust
pub fn move_layer_down(&mut self, index: usize, undo: &mut UndoHistory)
```

### Parameters

| Parameter | Type               | Description                         |
| --------- | ------------------ | ----------------------------------- |
| `index`   | `usize`            | Index of the layer to move downward |
| `undo`    | `&mut UndoHistory` | Undo history for the undo record    |

### Panics

Panics if `index >= pixels.len() - 1` — there is no layer below. The caller
(the UI) must ensure that `index < pixels.len() - 1` before calling.

### Behaviour

- Gets a mutable canvas via `self.canvas_mut()` and swaps layers with
  `canvas.pixels.swap(index, index + 1)`.
- Sets `current_layer = index + 1` (the layer moves with the swap).
- Pushes `UndoRecord::MoveLayer` onto the undo stack.
- Calls `canvas_mut().dirty_rect.request_full_blend()`.

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

### Signature

```rust
pub fn canvas_mut(&mut self) -> &mut Canvas
```

Returns a mutable reference to the underlying canvas. Uses `Arc::make_mut` to clone-on-write: when an async save holds a second `Arc` reference, the canvas is cloned before mutation; when no other references exist, this is a cheap `Arc::get_mut`.

### Returns

A `&mut Canvas` referencing the underlying canvas data.

### Panics

Does not panic in normal single-threaded use. `Arc::make_mut` would panic if the `Arc` had outstanding weak references with multiple strong references, but this codebase never creates weak references to the canvas.

---

## `Document::toggle_layer_visible(index, undo)`

```rust
pub fn toggle_layer_visible(&mut self, index: usize, undo: &mut UndoHistory)
```

Toggles the `visible` flag of the layer at `index`. Refuses to hide the last
remaining visible layer (at least one layer must stay visible).

**Parameters:**

- `index` — Index of the layer to toggle.
- `undo` — Undo history; a `ModifyLayer` record is pushed on successful toggle.

---

## `Document::set_layer_opacity(index, opacity, undo)`

```rust
pub fn set_layer_opacity(&mut self, index: usize, opacity: u8, undo: &mut UndoHistory)
```

Sets the opacity of the layer at `index`. A `ModifyLayer` record is pushed
onto the undo stack with the previous opacity and visibility values.

**Parameters:**

- `index` — Index of the layer to modify.
- `opacity` — New opacity value (0–255).
- `undo` — Undo history for the `ModifyLayer` record.

---

## `Document::rename_layer(index, name, undo)`

```rust
pub fn rename_layer(&mut self, index: usize, name: String, undo: &mut UndoHistory)
```

Renames the layer at `index` to the given `name`. Pushes a `ModifyLayer`
record onto the undo stack with the previous name, opacity, and visibility.

**Parameters:**

- `index` — Index of the layer to rename.
- `name` — New layer name.
- `undo` — Undo history for the `ModifyLayer` record.
