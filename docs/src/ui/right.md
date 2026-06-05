# right

Right properties panel. Implements `MyApp::show_right_panel` for colour
picking, brush configuration, undo strength, layer management, and save path.

## `MyApp::show_right_panel`

```rust
pub fn show_right_panel(&mut self, ui: &mut egui::Ui)
```

Renders a vertical panel containing the following sections:

### Settings

| Section                | Controls                                                                                                                                                | Bound field                                                                                                                                      |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Color Selector**     | Eyedropper toggle button + `color_edit_button_srgba`                                                                                                    | `tool_configuration.current_tool` / `current_color`                                                                                              |
| **Undo/Redo Strength** | `DragValue` (range 1–1000)                                                                                                                              | `ui_state.undo_redo_steps_multiplier`                                                                                                            |
| **Brush Settings**     | `DragValue` for radius (0–1000), checkboxes for Brush Preview, Alpha Overlay, Show Grid, Stabilize; sliders for Grid Size (1–500) and Smoothing (0–100) | `tool_configuration.radius`, `show_brush_preview`, `alpha_overlay`, `show_grid`, `grid_size`, `stabilization_enabled`, `stabilization_smoothing` |
| **Save Path**          | `text_edit_singleline`                                                                                                                                  | `document.savefile_path`                                                                                                                         |

### Layer management

An "Add Layer" button at the top of the layer section appends a new layer via
`Document::add_layer()`. Below it, a scrollable list shows each layer in a
`CollapsingHeader` labelled `Layer {i}`.

Each layer header contains four action buttons:

| Button         | Behaviour                                                                                                                                 |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| **Delete**     | Queues a `LayerAction::Delete(i)` that is processed after the layer iteration loop. The action is deferred to avoid simultaneous borrows. |
| **Move Up**    | Swaps with the layer above via `Document::move_layer_up(i)`. Disabled for `i == 0`.                                                       |
| **Move Down**  | Swaps with the layer below via `Document::move_layer_down(i)`. Disabled for the bottom layer.                                             |
| **Visibility** | Toggles the layer's `visible` flag via `Document::toggle_layer_visible(i, undo)`.                                                         |
| **Rename**     | Edits the layer name inline; the new name is sent as `LayerAction::Rename(i, name)` on focus loss.                                        |
| **Opacity**    | Slider (0–255) that updates the layer's opacity via `Document::set_layer_opacity(i, opacity, undo)`.                                      |

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
    ToggleVisible(usize),
    Rename(usize, String),
}
```
