# tests::asset_library

Tests for the generic [`Library<T>`] asset storage and [`AssetEntry`] trait, using a minimal `TestEntry` type to verify the generic machinery independently of brush/stamp specialisations.

## Test strategy

Each library operation (load, add, remove, select, get, entries, persistence, selected_mut) is exercised with empty and populated libraries to verify correct behaviour at boundaries.

## `load_from_disk_creates_dir`

Loading from a non-existent directory creates it and returns an empty library.

## `add_entry_increments_count`

Adding an entry increments the count and selects it.

## `add_multiple_preserves_order`

Adding multiple entries preserves insertion order; the first-added entry is at index 0, the second at index 1, and so on.

## `remove_last_clears_selection`

Removing the only entry in the library clears the selection and leaves the library empty.

## `remove_middle_preserves_order`

Removing a middle entry shifts later entries down and preserves the relative order of remaining entries.

## `select_switches_active`

Selecting by index switches which entry is returned by `selected()`.

## `select_out_of_bounds_noop`

Selecting with an out-of-bounds index leaves the current selection unchanged.

## `get_valid_index`

`get` with a valid index returns `Some(entry)`.

## `get_out_of_bounds_none`

`get` with an out-of-bounds index returns `None`.

## `entries_empty`

`entries()` on an empty library returns an empty slice.

## `entries_returns_all`

`entries()` returns all added entries after multiple insertions.

## `persistence_round_trip`

Entries added to the library survive a reload from disk, confirming serialisation and deserialisation round-trip works.

## `selected_mut_allows_mutation`

`selected_mut()` returns a mutable reference that allows modifying the selected entry's fields.

## `selected_mut_empty_none`

`selected_mut()` on an empty library returns `None`.

## `remove_out_of_bounds_noop`

Removing with an out-of-bounds index leaves the library contents unchanged.
