# file_io::load_import_manager

Async load/import orchestration: background-thread canvas deserialisation and
image import.

## Enums

### `LoadImportResult`

Result of an async load or import operation sent via channel. Uses `Vec<Layer>`
+ dimensions instead of `Canvas` directly so the data is `Send` (avoids non-Send
`TextureHandle`). The UI thread reconstructs a `Canvas` from the layers when
polled.

| Variant | Description |
| ------- | ----------- |
| `Loaded(Canvas, String)` | Canvas loaded from a `.splattercanvas` file, plus source path. |
| `Imported(Vec<Layer>, u32, u32)` | Image imported as a new canvas with layers and dimensions. |
| `ArchiveImported(Canvas)` | Canvas imported from a `.splatterarchive` file. |
| `Failed(String)` | Operation failed with an error message. |

## Structs

### `LoadImportManager`

Spawns background threads to read, deserialise, and import canvas files. Owns
the load/import channel pair and separate in-flight flags for load vs. import
operations.

| Field | Type | Description |
| ----- | ---- | ----------- |
| `load_import_sender` | `mpsc::Sender<LoadImportResult>` | Channel sender for results from background thread. |
| `load_import_receiver` | `mpsc::Receiver<LoadImportResult>` | Channel receiver for results on the UI thread. |
| `load_in_flight` | `bool` | `true` while an async load thread is running. |
| `import_in_flight` | `bool` | `true` while an async import thread is running. |

## Methods

### `LoadImportManager::new`

```rust
pub fn new() -> Self
```

Creates a new `LoadImportManager` with an internal channel pair. Both in-flight
flags start `false`.

Implements `Default`.

### `LoadImportManager::trigger_async_load`

```rust
pub fn trigger_async_load(&mut self, path: PathBuf)
```

Spawns a background thread to read and deserialise a `.splattercanvas` file.
Sends `LoadImportResult::Loaded` on success or `Failed` on error.

### `LoadImportManager::trigger_async_import`

```rust
pub fn trigger_async_import(&mut self, path: PathBuf)
```

Spawns a background thread to decode and import an image file as a new canvas.
Sends `LoadImportResult::Imported` on success or `Failed` on error.

### `LoadImportManager::trigger_async_import_archive`

```rust
pub fn trigger_async_import_archive(&mut self, path: PathBuf)
```

Spawns a background thread to read and deserialise a `.splatterarchive` file.
Sends `LoadImportResult::ArchiveImported` on success or `Failed` on error.

### `LoadImportManager::poll_load_import_results`

```rust
pub fn poll_load_import_results(
    &mut self,
    document: &mut Document,
    undo: &mut UndoHistory,
    error_list: &mut Vec<String>,
)
```

Polls for completed async load or import results and applies them. For a loaded
canvas, replaces the document canvas and sets the save path. For an imported
image, replaces the document canvas. Pushes errors to `error_list`.
