# top

Top menu bar panel. Implements `MyApp::show_top_panel` for file operations,
export, undo/redo, and application exit.

## `MyApp::show_top_panel`

```rust
pub fn show_top_panel(&mut self, ui: &mut egui::Ui) -> bool
```

Renders the horizontal toolbar at the top of the window containing:

| Button      | Behaviour                                                                                                                            |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| **Save**    | If `savefile_path` is empty, queues a Save dialog. Otherwise writes directly to the current path via `FileIO::save_to_current_path`. |
| **Load**    | Routes through `guard_unsaved(UnsavedWarningAction::Load)` to handle unsaved changes before opening file dialog.                     |
| **New**     | Routes through `guard_unsaved(UnsavedWarningAction::NewCanvas)` to handle unsaved changes before showing new-canvas dialog.          |
| **Export**  | Dropdown menu listing all 13 supported formats from `EXPORT_FORMATS`. Each entry queues `PendingFileAction::Export(i)`.              |
| **Import**  | Routes through `guard_unsaved(UnsavedWarningAction::Import)` to handle unsaved changes.                                              |
| **File**    | Dropdown menu with **Export Archive** (queues `ExportArchive`) and **Import Archive** (queues `ImportArchive`).                      |
| **Autosaves** | Opens the autosave directory (`{data_dir}/autosaves/`) in the OS file manager.                                                     |
| **Undo**    | Applies `UndoHistory::undo_step` with the current multiplier. Also responds to `Cmd+Z`.                                              |
| **Redo**    | Applies `UndoHistory::redo_step`. Also responds to `Cmd+Shift+Z` and `Cmd+Y`.                                                        |
| **Close**   | Sets the return value to `true`, signalling the app to quit.                                                                         |

### Returns

`true` if the Close button was pressed, indicating the application should exit.

### Keyboard shortcuts

#### Tool switching (single key, no modifier)

| Shortcut     | Tool              |
| ------------ | ----------------- |
| `S`          | Square            |
| `C`          | Circle            |
| `E`          | SquareEraser (toggle) |
| `Shift+E`    | CircleEraser      |
| `G`          | BucketFill        |
| `T`          | Stamp             |
| `B`          | CustomBrush       |
| `I`          | Eyedropper        |
| `H`          | Pan               |

#### File operations

| Shortcut        | Action           |
| --------------- | ---------------- |
| `Cmd+N`         | New canvas       |
| `Cmd+O`         | Open file        |
| `Cmd+S`         | Save             |
| `Cmd+Shift+S`   | Save As          |
| `Cmd+I`         | Import image     |
| `Cmd+E`         | Export PNG       |

#### Edit

| Shortcut      | Action           |
| ------------- | ---------------- |
| `Cmd+Z`       | Undo             |
| `Cmd+Shift+Z` | Redo             |
| `Cmd+Y`       | Redo (alternate) |

The Undo/Redo buttons and keyboard shortcuts are guarded by
`UndoHistory::can_undo()` / `can_redo()` — they no-op when the stack is empty.

### State effects

- Save and Load trigger `ui.ctx().request_repaint()` to ensure the file dialog
  appears promptly.
- Undo/Redo call `canvas_mut().dirty_rect.request_full_blend()` to force a full re-composite.


