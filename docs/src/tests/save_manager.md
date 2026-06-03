# tests::save_manager

Tests for `SaveManager` — async save orchestration, result polling.

## Test cases

| Test | Description |
| ---- | ----------- |
| `poll_save_results_autosave_clears_dirty` | Autosave result clears `dirty_since_last_autosave`. |
| `poll_save_results_manual_save_sets_path` | ManualSave result updates `savefile_path` and requests reblend. |
| `poll_save_results_failed_appends_error` | Failed result pushes error to error list. |
| `poll_save_results_no_messages_is_noop` | No messages results in no-op. |
| `poll_save_results_manual_save_empty_path` | Empty path manual save does not panic. |
| `save_to_current_path_empty_path_noop` | Saving with empty path is a no-op. |
| `save_to_current_path_non_empty_triggers_save` | Saving with a non-empty path spawns a thread without panic. |
| `trigger_async_save_writes_file` | Manual save actually writes the file to disk. |
| `autosave_directory_path` | Autosave directory path is `{data_dir}/autosaves`. |
