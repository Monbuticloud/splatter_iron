# file_io::dialog_manager

File-dialog state machine: queues native dialogs via `rfd` on background threads
and dispatches results back to the UI thread through mpsc channels.

## Enums

### `PendingFileAction`

A file-dialog action queued for execution on a background thread. The result is
received via channel at the start of a future frame.

| Variant         | Description                                                   |
| --------------- | ------------------------------------------------------------- |
| `Load`          | Open dialog filtered for `.splattercanvas` files.             |
| `Save`          | Save dialog with default name `canvas.splattercanvas`.        |
| `Import`        | Open dialog filtered for image files (`IMPORT_EXTENSIONS`).   |
| `Export(usize)` | Save dialog for the export format at `EXPORT_FORMATS[index]`. |
| `LoadStamp`     | Open dialog for image files; decodes the selection.           |
| `LoadBrush`     | Open dialog for `.abr`/`.gbr` brush files; parses on thread.  |
| `ExportArchive` | Save dialog for `.splatterarchive` files.                     |
| `ImportArchive` | Open dialog for `.splatterarchive` files.                     |

Derives `Clone`, `Copy`, `Debug`.

### `DialogResult`

Message sent from the file-dialog background thread to the UI thread.

| Variant                                       | Description                                          |
| --------------------------------------------- | ---------------------------------------------------- |
| `Picked(PathBuf)`                             | User selected a file path.                           |
| `StampPixels(Vec<Color32>, u32, u32, String)` | Decoded stamp image pixels + dimensions + name stem. |
| `BrushTips(Vec<BrushTip>)`                    | Parsed brush tips from an ABR/GBR file.              |
| `Error(String)`                               | An error occurred during a file operation.           |
| `Cancelled`                                   | User cancelled the dialog without selecting a file.  |

### `DispatchedAction`

An action decoded from a dialog result that the frame loop must dispatch
to the appropriate subsystem (save, load, import, or export).

| Variant                  | Description                                      |
| ------------------------ | ------------------------------------------------ |
| `Save(PathBuf)`          | Save the canvas to the given path.               |
| `Load(PathBuf)`          | Load a `.splattercanvas` file.                   |
| `Import(PathBuf)`        | Import an image file as a new canvas.            |
| `Export(usize, PathBuf)` | Export as image; usize indexes `EXPORT_FORMATS`. |
| `ExportArchive(PathBuf)` | Serialise and export a `.splatterarchive`.       |
| `ImportArchive(PathBuf)` | Import a `.splatterarchive` file.                |

## Structs

### `DialogManager`

Manages native file dialogs on background threads. Owns the dialog-channel pair,
the pending-action state machine, and temporary storage for loaded stamp/brush data.

| Field                 | Type                                       | Description                                                              |
| --------------------- | ------------------------------------------ | ------------------------------------------------------------------------ |
| `pending_file_action` | `Option<PendingFileAction>`                | File action queued for the next background thread iteration.             |
| `dialog_sender`       | `mpsc::Sender<DialogResult>`               | Channel sender for dispatching dialog requests to the background thread. |
| `dialog_receiver`     | `mpsc::Receiver<DialogResult>`             | Channel receiver for receiving dialog results on the UI thread.          |
| `loaded_stamp_data`   | `Option<(Vec<Color32>, u32, u32, String)>` | Decoded stamp image data, consumed by the app frame.                     |
| `loaded_brush_data`   | `Option<Vec<BrushTip>>`                    | Parsed brush tips, consumed by the app frame.                            |

## Methods

### `DialogManager::new`

```rust
pub fn new(
    dialog_sender: mpsc::Sender<DialogResult>,
    dialog_receiver: mpsc::Receiver<DialogResult>,
) -> Self
```

Creates a new `DialogManager` with an open channel pair. Stores the channels and
initialises `pending_file_action`, `loaded_stamp_data`, and `loaded_brush_data`
to `None`.

### `DialogManager::queue_file_action`

```rust
pub fn queue_file_action(&mut self, action: PendingFileAction)
```

Queues a file dialog action and spawns it on a background thread. Dispatched via
`rfd` to avoid macOS winit re-entrancy panics. Each `PendingFileAction` variant
opens the appropriate native dialog and sends the result back through
`dialog_sender`.

### `DialogManager::queue_load_direct`

```rust
pub fn queue_load_direct(&mut self, path: PathBuf)
```

Queues a direct file load without showing a dialog. Reuses the existing
`PendingFileAction::Load` handler by sending a synthetic `Picked` result through
the dialog channel.

### `DialogManager::poll_dialog_results`

```rust
pub fn poll_dialog_results(
    &mut self,
    error_list: &mut Vec<String>,
) -> Vec<DispatchedAction>
```

Drains the dialog result channel, handles `StampPixels`, `BrushTips`, `Cancelled`,
and `Error` internally, and returns a list of path-based `DispatchedAction` values
for the caller to dispatch.
