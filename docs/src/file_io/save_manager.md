# file_io::save_manager

Async save orchestration: background-thread serialisation for manual saves and
periodic autosaves.

## Enums

### `SaveKind`

Distinguishes an autosave from a manual save in the async save pipeline.

| Variant               | Description                                    |
| --------------------- | ---------------------------------------------- |
| `Autosave`            | Periodic autosave to `{data_dir}/autosaves/`.  |
| `ManualSave(PathBuf)` | Explicit user-initiated save to a chosen path. |

### `SaveResult`

Result of an async save operation sent via channel.

| Variant               | Description                              |
| --------------------- | ---------------------------------------- |
| `Autosave`            | Autosave completed successfully.         |
| `ManualSave(PathBuf)` | Manual save completed to the given path. |
| `Failed(String)`      | Save failed with an error message.       |

Derives `Debug`.

## Structs

### `SaveManager`

Spawns background threads to serialise and write canvas files. Owns the
save-result channel pair and the in-flight flag. The `app_local_data_directory`
is used to construct autosave paths.

| Field                      | Type                         | Description                                                                         |
| -------------------------- | ---------------------------- | ----------------------------------------------------------------------------------- |
| `save_result_sender`       | `mpsc::Sender<SaveResult>`   | Channel sender for results from background thread to UI thread.                     |
| `save_result_receiver`     | `mpsc::Receiver<SaveResult>` | Channel receiver for results on the UI thread.                                      |
| `app_local_data_directory` | `PathBuf`                    | Base path for autosave directory (`{data_dir}/autosaves/`).                         |
| `autosave_in_flight`       | `bool`                       | `true` when the most recent async save is an autosave. Used for status-bar display. |

## Methods

### `SaveManager::new`

```rust
pub fn new(
    save_result_sender: mpsc::Sender<SaveResult>,
    save_result_receiver: mpsc::Receiver<SaveResult>,
    app_local_data_directory: PathBuf,
) -> Self
```

Creates a new `SaveManager` with the given channel pair and data directory.
Initialises `autosave_in_flight` to `false`.

### `SaveManager::autosave_directory`

```rust
pub fn autosave_directory(&self) -> PathBuf
```

Returns the path to the autosave directory (`{data_dir}/autosaves/`).

### `SaveManager::trigger_async_save`

```rust
pub fn trigger_async_save(&mut self, document: &mut Document, kind: SaveKind)
```

Spawns a background thread to serialise and write the canvas to disk. The thread
clones the canvas to avoid blocking the UI. For autosaves the file name is a
timestamp under the autosave directory. Results are sent back via the save-result
channel.

### `SaveManager::save_to_current_path`

```rust
pub fn save_to_current_path(&mut self, document: &mut Document)
```

Convenience method that saves to the document's existing `savefile_path`
asynchronously. No-op if `savefile_path` is empty.

### `SaveManager::poll_save_results`

```rust
pub fn poll_save_results(
    &mut self,
    document: &mut Document,
    error_list: &mut Vec<String>,
)
```

Polls for completed async save results and updates state accordingly. Marks the
document as clean after autosave, sets the save path after manual save, pushes
errors to the error list, and resets `SaveState` to `Idle`.
