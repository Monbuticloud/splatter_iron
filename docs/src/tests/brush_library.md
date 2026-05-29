# tests/brush_library

Tests for `BrushLibrary` — add, remove, select, and persistence round-trip.

## `add_brush_increments_count`

Add one brush and verify it is selected and count increments to 1.

## `remove_brush_decrements_count`

Remove a brush and verify count decreases and selection clears.

## `select_switches_active_brush`

Select a specific brush by index; verify selected_index and name update.
