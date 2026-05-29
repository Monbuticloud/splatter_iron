# tests/brush_library

Tests for `BrushLibrary` — add, remove, select, and persistence round-trip.

## `add_brush_increments_count`

Add one brush and verify it is selected and count increments to 1.

## `remove_brush_decrements_count`

Remove a brush and verify count decreases and selection clears.

## `select_switches_active_brush`

Select a specific brush by index; verify selected_index and name update.

## `persistence_round_trip`

Persist to temp dir, reload, and verify entries survive.

## `remove_last_brush_clears_selection`

Remove the last brush clears selection to None.

## `remove_out_of_bounds_noop`

Remove with out-of-bounds index is a no-op.

## `select_out_of_bounds_noop`

Select with out-of-bounds index is a no-op.
