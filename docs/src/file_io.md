# file_io

Async file-dialog and save handling via mpsc channels. Manages save-to-current-path, save-as, load, and autosave workflows with result polling. File dialogs run on background threads via `rfd` to avoid macOS winit re-entrancy panics. Save operations clone the canvas and serialize/compress on a background thread, sending results back to the UI thread for processing.

## Enums

### `PendingFileAction`

A file-dialog action queued for execution on a background thread. The result is received via channel at the start of a future frame.

| Variant         | Description                                                                                                                                                                                                                                   |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Load`          | Opens a native `rfd::FileDialog` "open" filter for `.splattercanvas` files. Spawns a thread; on user selection, sends `DialogResult::Picked(path)` back through `dialog_sender`.                                                              |
| `Save`          | Opens a native `rfd::FileDialog` "save" dialog for `.splattercanvas` with the default filename `canvas.splattercanvas`. Spawns a thread; on user selection, sends `DialogResult::Picked(path)`.                                               |
| `Import`        | Opens a native `rfd::FileDialog` "open" dialog filtered for image files (`IMPORT_EXTENSIONS`). Spawns a thread; on user selection, sends `DialogResult::Picked(path)`.                                                                        |
| `Export(usize)` | Opens a native `rfd::FileDialog` "save" dialog for the export format at the given index into `EXPORT_FORMATS`. Sets default filename to `export.{primary_extension}`. Spawns a thread; on user selection, sends `DialogResult::Picked(path)`. |
| `LoadStamp`     | Opens a native `rfd::FileDialog` "open" dialog filtered for image files (`IMPORT_EXTENSIONS`). Spawns a thread; decodes the selected image and sends `DialogResult::StampPixels` or `DialogResult::Error`. |
| `LoadBrush`     | Opens a native `rfd::FileDialog` "open" dialog filtered for `.abr`/`.gbr` files. Spawns a thread; parses the file and sends `DialogResult::BrushTips` or `DialogResult::Error`. |
| `ExportArchive` | Opens a native `rfd::FileDialog` "save" dialog for `.splatterarchive` files. Spawns a thread; on user selection, sends result through the dialog channel for archive serialization. |
| `ImportArchive` | Opens a native `rfd::FileDialog` "open" dialog filtered for `.splatterarchive` files. Spawns a thread; on user selection, loads and deserializes the archive, replacing the current canvas. |

Derives `Clone`, `Copy`.

### `DialogResult`

Message sent from the file-dialog background thread to the UI thread after the user interacts with a native dialog.

| Variant                       | Description                                                                                                                                                      |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Picked(PathBuf)`             | User selected a file path via the native dialog. Sent through `dialog_sender` and received by `poll_dialog_results` on the next frame.                           |
| `StampPixels(Vec, u32, u32, String)` | Decoded stamp image pixels + dimensions + suggested name stem, sent from the background thread after a `LoadStamp` action.                                |
| `BrushTips(Vec<BrushTip>)`    | Parsed brush tips from an ABR/GBR file, sent from the background thread after a `LoadBrush` action.                                                              |
| `Error(String)`               | An error occurred during a file operation on the background thread. The string is a human-readable description.                                                   |
| `Cancelled`                   | User closed or cancelled the native dialog without selecting a file. Clears `pending_file_action`.                                                                  |

### `SaveKind`

Distinguishes an autosave from a manual save in the async save pipeline.

| Variant               | Description                                                                                                                                                                 |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Autosave`            | Periodic autosave triggered by the 2-minute timer in `UIState`. Saves to `{data_dir}/autosaves/{timestamp}.splattercanvas`. The resulting path is not surfaced to the user. |
| `ManualSave(PathBuf)` | Explicit user-initiated save to a chosen path. The `PathBuf` is the file path selected via dialog or the current `savefile_path`.                                           |

### `SaveResult`

Result sent back via channel when an async save completes. Received by `poll_save_results` on the UI thread.

| Variant               | Description                                                                                                                                                                     |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Autosave`            | Autosave completed successfully. `poll_save_results` sets `document.dirty_since_last_autosave = false` in response.                                                             |
| `ManualSave(PathBuf)` | Manual save completed to the given path. `poll_save_results` updates `document.savefile_path` and calls `request_full_blend` to trigger a re-composite.                                              |
| `ArchiveAutosave`     | Archive autosave completed successfully. `poll_save_results` resets the archive autosave timer in response.                                                                         |
| `Failed(String)`      | Save failed at the serialization stage. The string is a human-readable error message pushed to the error list. |

Derives `Debug`.

## Structs

### `FileIO`

Manages async file dialogs and save operations via background threads. Holds channel pairs for receiving dialog results and save outcomes, plus the app's local data directory path for autosaves.

| Field                      | Type                           | Description                                                                                                                                                                                      |
| -------------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `pending_file_action`      | `Option<PendingFileAction>`    | File action queued for the next background thread iteration. Set by `queue_file_action` before spawning the dialog thread; consumed by `poll_dialog_results` after a `DialogResult` is received. |
| `dialog_sender`            | `mpsc::Sender<DialogResult>`   | Channel sender for dispatching dialog results back from the background thread to the UI thread. Cloned into each dialog-spawned thread.                                                          |
| `dialog_receiver`          | `mpsc::Receiver<DialogResult>` | Channel receiver for polling dialog results on the UI thread. Polled once per frame in `poll_dialog_results`.                                                                                    |
| `save_result_sender`       | `mpsc::Sender<SaveResult>`     | Channel sender for dispatching save outcomes from the background save thread to the UI thread. Cloned into each save-spawned thread.                                                             |
| `save_result_receiver`     | `mpsc::Receiver<SaveResult>`   | Channel receiver for polling save results on the UI thread. Polled once per frame in `poll_save_results`.                                                                                        |
| `app_local_data_directory` | `PathBuf`                      | Base path for the autosave directory. Autosaves are written to `{app_local_data_directory}/autosaves/{timestamp}.splattercanvas`.                                                                |
| `loaded_stamp_data`        | `Option<(Vec<Color32>, u32, u32, String)>` | Decoded stamp image data, set when `poll_dialog_results` receives `StampPixels`. Consumed by the app frame loop.                                    |
| `loaded_brush_data`        | `Option<Vec<BrushTip>>`       | Parsed brush tips, set when `poll_dialog_results` receives `BrushTips`. Consumed by the app frame loop.                                               |
| `export_strategy`          | `Arc<dyn ExportStrategy + Send + Sync>` | Injected export strategy for writing image files. Defaults to `DefaultExportStrategy` which handles all 13 supported formats.                                                    |
| `export_result_sender`     | `mpsc::Sender<anyhow::Result<()>>` | Channel sender for export results from the background thread.                                                                                                                                  |
| `export_result_receiver`   | `mpsc::Receiver<anyhow::Result<()>>` | Channel receiver for export results on the UI thread. Polled in `poll_export_results`.                                                                                                      |
| `export_in_flight`         | `bool`                        | `true` while an async export thread is running.                                                                                                                                                  |
| `load_import_sender`       | `mpsc::Sender<LoadImportResult>` | Channel sender for load/import results from the background thread.                                                                                                                            |
| `load_import_receiver`     | `mpsc::Receiver<LoadImportResult>` | Channel receiver for load/import results on the UI thread. Polled in `poll_load_import_results`.                                                                                             |
| `load_in_flight`           | `bool`                        | `true` while an async load thread is running.                                                                                                                                                    |
| `import_in_flight`         | `bool`                        | `true` while an async import thread is running.                                                                                                                                                  |
| `autosave_in_flight`       | `bool`                        | `true` when the most recently triggered async save is an autosave. Used for status-bar display.                                                                                                 |
| `archive_autosave_in_flight` | `bool`                      | `true` while an archive autosave (`.splatterarchive`) is in flight.                                                                                                                              |

## Methods

### `FileIO::new`

```rust
pub fn new(
    dialog_sender: mpsc::Sender<DialogResult>,
    dialog_receiver: mpsc::Receiver<DialogResult>,
    save_result_sender: mpsc::Sender<SaveResult>,
    save_result_receiver: mpsc::Receiver<SaveResult>,
    app_local_data_directory: PathBuf,
    export_strategy: Arc<dyn ExportStrategy + Send + Sync>,
) -> Self

Constructor that stores the channel pairs, the app data directory, and the export strategy. Initialises `pending_file_action` to `None` and creates internal channels for export and load-import results.

**Parameters:**

- `dialog_sender` / `dialog_receiver` — Channel pair for file-dialog results (background thread → UI thread).
- `save_result_sender` / `save_result_receiver` — Channel pair for async save outcomes (background thread → UI thread).
- `app_local_data_directory` — Base path under which the `autosaves/` subdirectory is created.
- `export_strategy` — Strategy for writing file exports (e.g. `DefaultExportStrategy`), shared via `Arc` for cross-thread access.

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
  - `LoadStamp` — Open dialog, image extensions filter (`IMPORT_EXTENSIONS`), calls `pick_file()`. Decodes the selected image on a background thread and sends `StampPixels` or `Error`.
  - `LoadBrush` — Open dialog, `.abr`/`.gbr` filter, calls `pick_file()`. Parses the file on a background thread and sends `BrushTips` or `Error`.

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

Called once per frame (before egui layout) to process completed file dialog results. Drains the `dialog_receiver` channel with `try_recv()`, matching each received `DialogResult` variant:

- **`StampPixels`** — Stores decoded pixels, dimensions, and name in `self.loaded_stamp_data` for the frame loop to present a naming dialog. Clears `pending_file_action`.
- **`BrushTips`** — Stores parsed brush tips in `self.loaded_brush_data` for the frame loop. Clears `pending_file_action`.
- **`Cancelled`** — Clears `pending_file_action` without any other action.
- **`Error(msg)`** — Pushes `msg` to `error_list`. Clears `pending_file_action`.
- **`Picked(path)`** — Matches against `pending_file_action` to determine the operation.

**Parameters:**

- `document` — The `Document` to modify. For `Load` and `Import`, calls `document.replace_canvas()` with the deserialized/imported canvas. For `Save`, triggers an async save via `self.trigger_async_save()`.
- `undo` — The `UndoHistory` to reset on load/import. Passed to `replace_canvas()` to clear the undo/redo stack.
- `error_list` — A `Vec<String>` of error messages displayed to the user. Failures at any stage (file read, deserialization, import, export) are pushed here.

**Operation details per `Picked` action:**

- **Save:** Appends `CANVAS_EXTENSION` if missing. Calls `trigger_async_save(document, SaveKind::ManualSave(path))`.
- **Load:** Reads via `crate::files::load_canvas_from_path`, replaces the canvas, and sets `document.savefile_path` to the loaded file's path.
- **Import:** Calls `crate::files::import_image_as_canvas` and replaces the canvas (no save-path update).
- **Export:** Skips if `output_rgba` is empty. Appends the default extension if missing. Calls `trigger_async_export` with the canvas pixels, dimensions, and format index.
- **ExportArchive:** Appends `ARCHIVE_EXTENSION` (`.splatterarchive`) if missing. Clones the canvas and calls `trigger_async_export_archive`.
- **ImportArchive:** Calls `trigger_async_import_archive` with the selected path.

After processing, `pending_file_action` is consumed (set to `None`).

### `FileIO::trigger_async_save`

```rust
pub fn trigger_async_save(&mut self, document: &mut Document, kind: SaveKind)
```

Spawns a background thread to serialize and write the canvas to disk. Clones the canvas to avoid borrowing the `Document` across threads. The thread pipeline is:

1. `crate::files::save_canvas_to_path(&canvas, &path)` — streams JSON directly
   through zstd compression into the file. No intermediate `Vec<u8>` allocations.
2. Sends a `SaveResult` back via `save_result_sender`.

**Parameters:**

- `document` — The document whose canvas is cloned and saved. The entire `Canvas` struct is cloned (`document.canvas.clone()`), so the UI thread can continue rendering immediately.
- `kind` — Determines the save path:
  - `Autosave` — Path is `{app_local_data_directory}/autosaves/{timestamp}.splattercanvas` where timestamp uses `AUTOSAVE_DATE_FORMAT` (`%Y-%m-%d_%H-%M-%S`).
  - `ManualSave(path)` — Uses the provided `PathBuf` directly.

**Failure modes:** Returns `SaveResult::Failed` with a descriptive string if serialization or file write fails. The error is prefixed with `"Serialisation failed: "`.

### `FileIO::save_to_current_path`

```rust
pub fn save_to_current_path(&mut self, document: &mut Document)
```

Convenience method that saves to the document's existing `savefile_path` asynchronously. No-op if `savefile_path` is empty (i.e. the document has never been saved or loaded from a file).

**Parameters:**

- `document` — The document whose canvas is saved. Delegates to `self.trigger_async_save(document, SaveKind::ManualSave(PathBuf::from(&document.savefile_path)))`.

Useful for keyboard shortcuts (e.g. Ctrl+S) that re-save to the same path without opening a dialog. If the document has no path yet, the caller should first call `queue_file_action(PendingFileAction::Save)` to prompt the user.

### `FileIO::poll_save_results`

```rust
pub fn poll_save_results(&mut self, document: &mut Document, error_list: &mut Vec<String>)
```

Called once per frame to process completed async save results. Drains the `save_result_receiver` channel with `try_recv()` and updates document state or pushes errors accordingly.

**Parameters:**

- `document` — The `Document` to update based on the save outcome:
  - `Autosave` → Sets `document.dirty_since_last_autosave = false`, marking the autosave as up-to-date.
  - `ManualSave(path)` → Sets `document.savefile_path = path.display().to_string()`, sets `document.dirty_since_last_autosave = false`, and calls `document.canvas_mut().dirty_rect.request_full_blend()` to trigger a full re-composite.
- `error_list` — A `Vec<String>` where `Failed(message)` results are pushed as `"Save failed: {message}"`.

Non-blocking: uses `try_recv()` so it will not stall the frame if no save has completed.

### `FileIO::poll_export_results`

```rust
pub fn poll_export_results(&mut self, error_list: &mut Vec<String>) -> bool
```

Called once per frame to check for completed async export operations. Drains the `export_result_receiver` channel. Returns `true` if any export completed (success or failure).

**Parameters:**

- `error_list` — A `Vec<String>` where `Err` results are pushed as `"Export failed: {message}"`.

### `FileIO::poll_load_import_results`

```rust
pub fn poll_load_import_results(
    &mut self,
    document: &mut Document,
    undo: &mut UndoHistory,
    error_list: &mut Vec<String>,
)
```

Called once per frame to process completed async load/import operations. Drains the `load_import_receiver` channel with `try_recv()`.

**Parameters:**

- `document` — The `Document` to modify with the loaded/imported canvas.
- `undo` — The `UndoHistory` to clear on load/import.
- `error_list` — Error messages from failed operations are pushed here.

### `FileIO::queue_load_direct`

```rust
pub fn queue_load_direct(&mut self, path: PathBuf)
```

Queues a load operation for a specific path without opening a file dialog. Spawns a thread that reads and deserialises the canvas, then sends the result via the load-import channel.

### `FileIO::trigger_async_load`

```rust
pub fn trigger_async_load(&mut self, path: PathBuf)
```

Spawns a background thread to load a `.splattercanvas` file and deserialise it into a `Canvas`. Sends `LoadImportResult::Loaded(Canvas)` on success or `LoadImportResult::Failed(String)` on error via the load-import channel.

### `FileIO::trigger_async_import`

```rust
pub fn trigger_async_import(&mut self, path: PathBuf)
```

Spawns a background thread to import an image file as a new `Canvas`. Sends `LoadImportResult::Imported(Canvas)` on success or `LoadImportResult::Failed(String)` on error.

### `FileIO::trigger_async_export`

```rust
pub fn trigger_async_export(
    &mut self,
    canvas: &Canvas,
    path: PathBuf,
    format_idx: usize,
)
```

Spawns a background thread to export the canvas as an image in the given format. Sends `Ok(())` on success or `Err(...)` via the export channel.

### `FileIO::trigger_async_export_archive`

```rust
pub fn trigger_async_export_archive(&mut self, canvas: Canvas, path: PathBuf)
```

Spawns a background thread to serialise the canvas as a `.splatterarchive` file. Sends the result via the load-import channel.

### `FileIO::trigger_async_import_archive`

```rust
pub fn trigger_async_import_archive(&mut self, path: PathBuf)
```

Spawns a background thread to read and deserialise a `.splatterarchive` file. Sends `LoadImportResult::ArchiveImported(Canvas)` on success.

### `FileIO::trigger_async_autosave_archive`

```rust
pub fn trigger_async_autosave_archive(&mut self, document: &Document)
```

Spawns a background thread to save an archive autosave (`.splatterarchive`) with timestamped name to the autosave directory. Sends `SaveResult::ArchiveAutosave` on success.

### `FileIO::autosave_directory`

```rust
pub fn autosave_directory(&self) -> PathBuf
```

Returns the path to the autosave subdirectory under `app_local_data_directory`.


