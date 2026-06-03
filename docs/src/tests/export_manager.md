# tests::export_manager

Tests for `ExportManager` — async export orchestration, result polling.

## Test cases

| Test | Description |
| ---- | ----------- |
| `poll_export_results_success_returns_true` | Successful export result returns `true` and clears in-flight flag. |
| `poll_export_results_error_appends` | Error result returns `true` and pushes to error list. |
| `poll_export_results_no_message_returns_false` | No result returns `false` with no errors. |
