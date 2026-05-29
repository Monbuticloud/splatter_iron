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

## `get_valid_index`

Retrieve a stamp by valid index returns Some with correct name.

## `get_out_of_bounds_returns_none`

Get with out-of-bounds index returns None.

## `entries_returns_all`

Entries returns a slice matching internal state with correct ordering.

## `entries_empty_library`

Entries on an empty library returns an empty slice.

## `selected_mut_allows_mutation`

Selected_mut allows mutation of the selected stamp's fields.
