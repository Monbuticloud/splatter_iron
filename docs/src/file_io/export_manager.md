# file_io::export_manager

Async export orchestration: background-thread image encoding and archive
serialisation.

## Structs

### `ExportManager`

Spawns background threads to encode and write exported images or archives. Owns
the export channel pair, the pluggable `ExportStrategy`, and the in-flight flag.
Used by the frame loop to trigger exports and poll for completions.

| Field | Type | Description |
| ----- | ---- | ----------- |
| `export_strategy` | `Arc<dyn ExportStrategy + Send + Sync>` | Injected export strategy for writing image files. |
| `export_result_sender` | `mpsc::Sender<anyhow::Result<()>>` | Channel sender for export results from background thread. |
| `export_result_receiver` | `mpsc::Receiver<anyhow::Result<()>>` | Channel receiver for export results on the UI thread. |
| `export_in_flight` | `bool` | `true` while an async export thread is running. |

## Methods

### `ExportManager::new`

```rust
pub fn new(export_strategy: Arc<dyn ExportStrategy + Send + Sync>) -> Self
```

Creates a new `ExportManager` with an internal channel pair and the given export
strategy. Initialises `export_in_flight` to `false`.

### `ExportManager::trigger_async_export`

```rust
pub fn trigger_async_export(
    &mut self,
    premultiplied_rgba: Arc<Vec<u8>>,
    width: u32,
    height: u32,
    path: PathBuf,
)
```

Spawns a background thread to encode and write the exported image using the
configured export strategy.

### `ExportManager::trigger_async_export_archive`

```rust
pub fn trigger_async_export_archive(&mut self, canvas: Canvas, path: PathBuf)
```

Spawns a background thread to serialise and write an xz-compressed
`.splatterarchive` file.

### `ExportManager::poll_export_results`

```rust
pub fn poll_export_results(&mut self, error_list: &mut Vec<String>) -> bool
```

Polls for completed async export results. Returns `true` if an export result was
processed (success or failure). On failure, pushes the error message into
`error_list`.
