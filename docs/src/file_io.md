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

## Structs

### `FileIO`

Manages async file dialogs and save operations via background threads. Holds channel pairs for receiving dialog results and save outcomes, plus the app's local data directory path for autosaves.

| Field | Type | Description |
|-------|------|-------------|
| `pending_file_action` | `Option<PendingFileAction>` | File action queued for the next background thread iteration. Set by `queue_file_action` before spawning the dialog thread; consumed by `poll_dialog_results` after a `DialogResult` is received. |
| `dialog_sender` | `mpsc::Sender<DialogResult>` | Channel sender for dispatching dialog results back from the background thread to the UI thread. Cloned into each dialog-spawned thread. |
| `dialog_receiver` | `mpsc::Receiver<DialogResult>` | Channel receiver for polling dialog results on the UI thread. Polled once per frame in `poll_dialog_results`. |
| `save_result_sender` | `mpsc::Sender<SaveResult>` | Channel sender for dispatching save outcomes from the background save thread to the UI thread. Cloned into each save-spawned thread. |
| `save_result_receiver` | `mpsc::Receiver<SaveResult>` | Channel receiver for polling save results on the UI thread. Polled once per frame in `poll_save_results`. |
| `app_local_data_directory` | `PathBuf` | Base path for the autosave directory. Autosaves are written to `{app_local_data_directory}/autosaves/{timestamp}.splattercanvas`. |

## Methods

### `FileIO::new`

```rust
pub fn new(
    dialog_sender: mpsc::Sender<DialogResult>,
    dialog_receiver: mpsc::Receiver<DialogResult>,
    save_result_sender: mpsc::Sender<SaveResult>,
    save_result_receiver: mpsc::Receiver<SaveResult>,
    app_local_data_directory: PathBuf,
) -> Self
```

Constructor that stores the two channel pairs and the app data directory. Initializes `pending_file_action` to `None`.

**Parameters:**
- `dialog_sender` / `dialog_receiver` — Channel pair for file-dialog results (background thread → UI thread).
- `save_result_sender` / `save_result_receiver` — Channel pair for async save outcomes (background thread → UI thread).
- `app_local_data_directory` — Base path under which the `autosaves/` subdirectory is created.

**Returns:** A fully initialized `FileIO` with no pending action. The channels are typically created by the caller (e.g. `app.rs`) via `mpsc::channel()` before passing them into `new`.

### `FileIO::queue_file_action`

```rust
pub fn queue_file_action(&mut self, action: PendingFileAction)
```

Queues a file dialog action and spawns it on a background thread. Dispatched via `rfd` to avoid macOS winit re-entrancy panics. The method stores the action in `pending_file_action` and spawns a thread that opens the appropriate native dialog. When the user picks a file (or cancels), the result is sent back via `dialog_sender`.

**Parameters:**
- `action` — The dialog action to perform. Each variant opens a different dialog:
  - `Load` — Open dialog, `.splattercanvas` filter, calls `rfd::FileDialog::pick_file()`.
  - `Save` — Save dialog, `.splattercanvas` filter, default name `canvas.splattercanvas`, calls `save_file()`.
  - `Import` — Open dialog, image extensions filter (`IMPORT_EXTENSIONS`), calls `pick_file()`.
  - `Export(index)` — Save dialog, export format filter from `EXPORT_FORMATS[index]`, default name `export.{ext}`, calls `save_file()`.

If the user cancels the dialog, no result is sent and `pending_file_action` remains set (consumed only when a `DialogResult` arrives).

### `FileIO::poll_dialog_results`

```rust
pub fn poll_dialog_results(
    &mut self,
    document: &mut Document,
    undo: &mut UndoHistory,
    error_list: &mut Vec<String>,
)
```

Called once per frame (before egui layout) to process completed file dialog results. Drains the `dialog_receiver` channel with `try_recv()`, matching each received `DialogResult::Picked(path)` against the `pending_file_action` to determine the operation.

**Parameters:**
- `document` — The `Document` to modify. For `Load` and `Import`, calls `document.replace_canvas()` with the deserialized/imported canvas. For `Save`, triggers an async save via `self.trigger_async_save()`.
- `undo` — The `UndoHistory` to reset on load/import. Passed to `replace_canvas()` to clear the undo/redo stack.
- `error_list` — A `Vec<String>` of error messages displayed to the user. Failures at any stage (file read, deserialization, import, export) are pushed here.

**Operation details per action:**
- **Save:** Appends `CANVAS_EXTENSION` if missing. Calls `trigger_async_save(document, SaveKind::ManualSave(path))`.
- **Load:** Reads via `crate::files::load_data_from_file`, deserializes via `load_app_from_data`, replaces the canvas, and sets `document.savefile_path` to the loaded file's path.
- **Import:** Calls `crate::files::import_image_as_canvas` and replaces the canvas (no save-path update).
- **Export:** Skips if `output_rgba` is empty. Appends the default extension if missing. Calls `crate::files::export_as_image` with the format and dimensions from the canvas.

After processing, `pending_file_action` is consumed (set to `None`).

### `FileIO::trigger_async_save`

```rust
pub fn trigger_async_save(&self, document: &Document, kind: SaveKind)
```

Spawns a background thread to serialize and write the canvas to disk. Clones the canvas to avoid borrowing the `Document` across threads. The thread pipeline is:

1. `crate::files::save_canvas_to_bytes(&canvas)` — serializes to zstd-compressed JSON bytes.
2. `crate::files::save_bytes_to_file(&data, &path)` — writes bytes to disk.
3. Sends a `SaveResult` back via `save_result_sender`.

**Parameters:**
- `document` — The document whose canvas is cloned and saved. The entire `Canvas` struct is cloned (`document.canvas.clone()`), so the UI thread can continue rendering immediately.
- `kind` — Determines the save path:
  - `Autosave` — Path is `{app_local_data_directory}/autosaves/{timestamp}.splattercanvas` where timestamp uses `AUTOSAVE_DATE_FORMAT` (`%Y-%m-%d_%H-%M-%S`).
  - `ManualSave(path)` — Uses the provided `PathBuf` directly.

**Failure modes:** Returns `SaveResult::Failed` with a descriptive string if serialization fails (stage 1) or file write fails (stage 2). The error is prefixed with `"Serialisation failed: "` or `"Write failed: "` respectively.

### `FileIO::save_to_current_path`

```rust
pub fn save_to_current_path(&self, document: &Document)
```

Convenience method that saves to the document's existing `savefile_path` asynchronously. No-op if `savefile_path` is empty (i.e. the document has never been saved or loaded from a file).

**Parameters:**
- `document` — The document whose canvas is saved. Delegates to `self.trigger_async_save(document, SaveKind::ManualSave(PathBuf::from(&document.savefile_path)))`.

Useful for keyboard shortcuts (e.g. Ctrl+S) that re-save to the same path without opening a dialog. If the document has no path yet, the caller should first call `queue_file_action(PendingFileAction::Save)` to prompt the user.

