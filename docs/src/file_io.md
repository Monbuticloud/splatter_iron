# file_io

Async file-dialog and save handling via mpsc channels. Manages save-to-current-path, save-as, load, and autosave workflows with result polling. File dialogs run on background threads via `rfd` to avoid macOS winit re-entrancy panics. Save operations clone the canvas and serialize/compress on a background thread, sending results back to the UI thread for processing.

## Enums

### `PendingFileAction`

A file-dialog action queued for execution on a background thread. The result is received via channel at the start of a future frame.

| Variant | Description |
|---------|-------------|
| `Load` | Opens a native `rfd::FileDialog` "open" filter for `.splattercanvas` files. Spawns a thread; on user selection, sends `DialogResult::Picked(path)` back through `dialog_sender`. |
| `Save` | Opens a native `rfd::FileDialog` "save" dialog for `.splattercanvas` with the default filename `canvas.splattercanvas`. Spawns a thread; on user selection, sends `DialogResult::Picked(path)`. |
| `Import` | Opens a native `rfd::FileDialog` "open" dialog filtered for image files (`IMPORT_EXTENSIONS`). Spawns a thread; on user selection, sends `DialogResult::Picked(path)`. |
| `Export(usize)` | Opens a native `rfd::FileDialog` "save" dialog for the export format at the given index into `EXPORT_FORMATS`. Sets default filename to `export.{primary_extension}`. Spawns a thread; on user selection, sends `DialogResult::Picked(path)`. |

Derives `Clone`, `Copy`.

### `DialogResult`

Message sent from the file-dialog background thread to the UI thread after the user interacts with a native dialog.

| Variant | Description |
|---------|-------------|
| `Picked(PathBuf)` | User selected a file path via the native dialog. Sent through `dialog_sender` and received by `poll_dialog_results` on the next frame. |

### `SaveKind`

Distinguishes an autosave from a manual save in the async save pipeline.

| Variant | Description |
|---------|-------------|
| `Autosave` | Periodic autosave triggered by the 2-minute timer in `UIState`. Saves to `{data_dir}/autosaves/{timestamp}.splattercanvas`. The resulting path is not surfaced to the user. |
| `ManualSave(PathBuf)` | Explicit user-initiated save to a chosen path. The `PathBuf` is the file path selected via dialog or the current `savefile_path`. |

### `SaveResult`

Result sent back via channel when an async save completes. Received by `poll_save_results` on the UI thread.

| Variant | Description |
|---------|-------------|
| `Autosave` | Autosave completed successfully. `poll_save_results` sets `document.dirty_since_last_autosave = false` in response. |
| `ManualSave(PathBuf)` | Manual save completed to the given path. `poll_save_results` updates `document.savefile_path` and sets `render_next_frame = true`. |
| `Failed(String)` | Save failed at either serialization (`save_canvas_to_bytes`) or file-write (`save_bytes_to_file`) stage. The string is a human-readable error message pushed to the error list. |

Derives `Debug`.
