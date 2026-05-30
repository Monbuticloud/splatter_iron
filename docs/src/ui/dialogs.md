# ui::dialogs

Modal and overlay dialog windows shown on top of the canvas.

## Methods

| Method                        | Purpose                                                    |
| ----------------------------- | ---------------------------------------------------------- |
| `show_error_window()`         | Centred error list with Dismiss / Copy / Dismiss All       |
| `show_large_canvas_warning()` | Confirmation dialog when canvas exceeds 500 MB memory      |
| `show_delete_layer_dialog()`  | Delete-layer confirmation with layer name display          |
| `show_new_canvas_dialog()`    | Preset sizes (XS–XL) + custom width/height sliders         |
| `show_unsaved_changes_warning()` | Save / Don't Save / Cancel modal                       |
| `guard_unsaved()`             | Route a destructive action through the unsaved-changes check |
| `execute_unsaved_action()`    | Execute a deferred destructive action after user resolves  |
| `show_stamp_naming_dialog()`  | Name prompt for imported stamp images                      |
| `show_brush_naming_dialog()`  | Batch name editor for imported brush tips                  |
| `show_toast()`                | Transient notification (2-second auto-dismiss)             |
| `show_progress_indicator()`   | Spinner overlay for in-flight async operations             |

All methods are `pub(crate)` and called from `ui()` in `app/mod.rs`.
