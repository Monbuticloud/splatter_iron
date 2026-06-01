# tests::asset_library

Tests for the generic [`Library<T>`] asset storage and [`AssetEntry`] trait, using a minimal `TestEntry` type to verify the generic machinery independently of brush/stamp specialisations.

## Test strategy

Each library operation (load, add, remove, select, get, entries, persistence, selected_mut) is exercised with empty and populated libraries to verify correct behaviour at boundaries.

## `load_from_disk_creates_dir`

Loading from a non-existent directory creates it and returns an empty library.

## `add_entry_increments_count`

Adding an entry increments the count and selects it.
