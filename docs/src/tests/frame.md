# tests::frame

Tests for frame-lifecycle methods in `src/app/frame.rs`. Covers
`handle_autosave`, `poll_file_results`, `update_render_state`,
`sync_gpu_texture`, and `recreate_gpu_texture`.

## Test cases

| Test                                                 | Description                                         |
| ---------------------------------------------------- | --------------------------------------------------- |
| `handle_autosave_skipped_when_not_dirty`             | Autosave does nothing when canvas is clean.         |
| `handle_autosave_triggers_when_dirty_and_elapsed`    | Autosave fires when dirty and interval elapsed.     |
| `handle_autosave_skipped_when_not_enough_time`       | Autosave skips when interval not yet elapsed.       |
| `handle_autosave_no_panic_with_unconnected_channels` | Autosave does not panic with disconnected channels. |
