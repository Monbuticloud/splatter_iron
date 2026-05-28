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
