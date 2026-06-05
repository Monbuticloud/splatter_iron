# tests::dialog_manager

Tests for `DialogManager` — dialog queueing, `DispatchedAction` dispatching,
stamp/brush data storage.

## Test cases

| Test                                                 | Description                                                       |
| ---------------------------------------------------- | ----------------------------------------------------------------- |
| `poll_dialog_results_save_returns_dispatched_action` | Save `Picked` returns `DispatchedAction::Save`.                   |
| `poll_dialog_results_load_returns_dispatched_action` | Load `Picked` returns `DispatchedAction::Load`.                   |
| `poll_dialog_results_mismatched_pending_skips`       | Mismatched pending action is skipped gracefully.                  |
| `poll_dialog_results_stamp_pixels_sets_loaded`       | `StampPixels` result populates `loaded_stamp_data`.               |
| `poll_dialog_results_error_appends`                  | `Error` result pushes to error list.                              |
| `poll_dialog_results_cancelled_clears_pending`       | `Cancelled` result clears pending action.                         |
| `queue_file_action_save_sets_pending`                | `queue_file_action(Save)` sets `pending_file_action`.             |
| `queue_file_action_load_stamp_sets_pending`          | `queue_file_action(LoadStamp)` sets pending.                      |
| `queue_file_action_export_sets_pending`              | `queue_file_action(Export(0))` sets pending.                      |
| `queue_file_action_export_archive_sets_pending`      | `queue_file_action(ExportArchive)` sets pending.                  |
| `queue_file_action_import_archive_sets_pending`      | `queue_file_action(ImportArchive)` sets pending.                  |
| `poll_dialog_results_export_archive_returns_action`  | ExportArchive `Picked` returns `DispatchedAction::ExportArchive`. |
| `poll_dialog_results_save_appends_extension`         | Save without extension appends `.splattercanvas`.                 |
