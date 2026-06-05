# ui::panels

Panel layout coordinator: renders all four egui panels in the correct order.

## Methods

| Method          | Purpose                                                                                                                                                                  |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `show_panels()` | Renders top (`show_top_panel`), left (`show_left_panel`), right (`show_right_panel`), and centre (`show_central_panel`) panels. Returns whether the user triggered quit. |

`pub(crate)`, called from `ui()` in `app/mod.rs`.
