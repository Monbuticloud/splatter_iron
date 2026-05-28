# right

Right properties panel. Implements `MyApp::show_right_panel` for colour
picking, brush configuration, undo strength, layer management, and save path.

## `MyApp::show_right_panel`

```rust
pub fn show_right_panel(&mut self, ui: &mut egui::Ui)
```

Renders a vertical panel containing the following sections:

### Settings

| Section | Controls | Bound field |
|---------|----------|-------------|
| **Color Selector** | `color_edit_button_srgba` | `tool_configuration.current_color` |
| **Undo/Redo Strength** | `DragValue` (range 1–1000) | `tool_configuration.undo_redo_steps_multiplier` |
| **Brush Settings** | `DragValue` for radius (0–350), checkbox for Brush Preview, checkbox for Alpha Overlay | `tool_configuration.radius`, `show_brush_preview`, `alpha_overlay` |
| **Save Path** | `text_edit_singleline` | `document.savefile_path` |

### Layer management

An "Add Layer" button at the top of the layer section appends a new layer via
`Document::add_layer()`. Below it, a scrollable list shows each layer in a
`CollapsingHeader` labelled `Layer {i}`.

Each layer header contains four action buttons:

| Button | Behaviour |
|--------|-----------|
| **Delete** | First click marks the layer as pending deletion. A second consecutive click on the same layer (confirmed by `ui.pending_layer_for_deletion`) deletes it, provided at least one layer would remain. |
| **Move Up** | Swaps with the layer above via `Document::move_layer_up(i)`. Disabled for `i == 0`. |
| **Move Down** | Swaps with the layer below via `Document::move_layer_down(i)`. Disabled for the bottom layer. |
| **Select** | Sets `document.current_layer = i` via `Document::select_layer(i)`. |

The currently selected layer shows a "Currently Selected" label. Deleting a
layer that is also the currently selected layer automatically adjusts
`current_layer` to stay valid.

### LayerAction enum

Layer actions are deferred through a `LayerAction` enum to avoid simultaneous
borrows of `self.document.canvas.pixels` (for the layer list iteration) and
`self` (for mutation). The enum is processed after the iteration completes:

```rust
enum LayerAction {
    Delete(usize),
    MoveUp(usize),
    MoveDown(usize),
    Select(usize),
}
```

### Deletion confirmation

The delete action requires a two-click confirmation: the first click sets
`ui.pending_layer_for_deletion = Some(index)`; the second click (with the
same index) performs the deletion. Any other click (tool selection, drag,
another layer's button) resets `pending_layer_for_deletion` to `None`.
