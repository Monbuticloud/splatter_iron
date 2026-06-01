# tests::persistence

Tests for config persistence — recent files ordering/dedup/cap, config.json
save/load round-trips, and autosave interval guard.

## Test strategy

- Push recent files: empty noop, front-insertion, dedup, 10-entry cap.
- Config path construction.
- Save/load round-trip with tool config and with empty recent files.
- Autosave guard: does not save before interval, does save after.

## `push_recent_file_empty_path_noop`

Pushing an empty `PathBuf` is a no-op — recent files remain empty.

## `push_recent_file_inserts_front`

Pushing paths inserts them at position 0, preserving stack order.

## `push_recent_file_dedup`

Pushing a duplicate path moves it to position 0 and removes the older entry.

## `push_recent_file_truncates_at_ten`

Pushing 11 paths caps the list at 10, retaining the most recent 10.

## `config_path_ends_with_config_json`

Verifies `config_path()` returns a path ending in `config.json` inside the data directory.

## `save_config_roundtrip`

Writes a config with `CurrentTool::Circle`, red color, and a recent file; reads it back and asserts all fields match.
