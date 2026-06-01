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
