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
