# left

Left tool palette panel. Implements `MyApp::show_left_panel` for tool
selection.

## `MyApp::show_left_panel`

```rust
pub fn show_left_panel(&mut self, ui: &mut egui::Ui)
```

Renders a vertical list of tool selection buttons using egui's
`selectable_value` widget, which combines radio-button semantics with
visual highlighting.

### Tools

| Order | Tool          | `CurrentTool` variant |
| ----- | ------------- | --------------------- |
| 1     | Square Tool   | `Square`              |
| 2     | Circle Tool   | `Circle`              |
| 3     | Square Eraser | `SquareEraser`        |
| 4     | Circle Eraser | `CircleEraser`        |
| 5     | Bucket Fill   | `BucketFill`          |

### Visual styling

The panel temporarily overrides `ui.visuals().selection.bg_fill` to a deep
purple (`rgb(128, 0, 128)`) so the active tool stands out against both dark
and light egui themes. The original selection colour is restored after
rendering the buttons.

### State effects

Sets `self.tool_configuration.current_tool` to the selected variant. No
return value or side effects beyond updating the tool selection.

## `Stamp Tool button`

Selects CurrentTool::Stamp. When active, displays the stamp gallery below the tool buttons showing all entries from StampLibrary with thumbnails.

## `CustomBrush Tool button`

Selects CurrentTool::CustomBrush. When active, displays the brush gallery below the tool buttons showing all entries from BrushLibrary with thumbnails.

## `Stamp gallery`

When Stamp tool is selected, renders a vertical gallery of stamp thumbnails. Each entry shows the stamp image and name. Clicking selects that stamp. Above the gallery are tint mode and sampling mode combo selectors.
