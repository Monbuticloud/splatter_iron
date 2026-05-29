# top

Top menu bar panel. Implements `MyApp::show_top_panel` for file operations,
export, undo/redo, and application exit.

## `MyApp::show_top_panel`

```rust
pub fn show_top_panel(&mut self, ui: &mut egui::Ui) -> bool
```

Renders the horizontal toolbar at the top of the window containing:

| Button     | Behaviour                                                                                                                            |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| **Save**   | If `savefile_path` is empty, queues a Save dialog. Otherwise writes directly to the current path via `FileIO::save_to_current_path`. |
| **Load**   | Queues a Load file dialog via `PendingFileAction::Load`.                                                                             |
| **New**    | Opens the new-canvas dialog (`ui.show_new_canvas_dialog = true`).                                                                    |
| **Export** | Dropdown menu listing all 13 supported formats from `EXPORT_FORMATS`. Each entry queues `PendingFileAction::Export(i)`.              |
| **Import** | Queues an Import file dialog (`PendingFileAction::Import`).                                                                          |
| **Undo**   | Applies `UndoHistory::undo_step` with the current multiplier. Also responds to `Cmd+Z`.                                              |
| **Redo**   | Applies `UndoHistory::redo_step`. Also responds to `Cmd+Shift+Z` and `Cmd+Y`.                                                        |
| **Close**  | Sets the return value to `true`, signalling the app to quit.                                                                         |

### Returns

`true` if the Close button was pressed, indicating the application should exit.

### Keyboard shortcuts

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
- Undo/Redo set `canvas.render_next_frame = true` to force a texture re-render.

## `Cmd+N (New)`

Opens the new-canvas dialog (sets show_new_canvas_dialog = true).
