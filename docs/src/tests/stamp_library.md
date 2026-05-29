# tests/stamp_library

Tests for `StampLibrary` — add, remove, select, and persistence round-trip.

## `add_stamp_increments_count`

Add one stamp and verify it is selected and count increments to 1.

## `remove_stamp_decrements_count`

Remove a stamp and verify count decreases and selection clears.

## `select_switches_active_stamp`

Select a specific stamp by index; verify selected_index and name update.

## `persistence_round_trip`

Persist to temp dir, reload, and verify entries survive.

## `remove_last_stamp_clears_selection`

Remove the last stamp clears selection to None.
