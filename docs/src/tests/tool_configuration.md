# tests::tool_configuration

Tests for `ToolConfiguration` — default values and field consistency.

## Test strategy

- Confirm that the default configuration matches the documented initial state for the application.
- Verify that optional fields (`previous_tool`, `previous_cursor_position`, `stamp_image`) are `None` by default.

## `default_values_match_expected`

The default `ToolConfiguration` uses the `Square` tool, white colour, radius 100, `alpha_overlay = false`, `show_brush_preview = true`, and `undo_redo_steps_multiplier = 1`.

## `default_optional_fields_are_none`

`previous_tool`, `previous_cursor_position`, and `stamp_image` are all `None` by default; `stamp_tinted` is `false`.
