# tests::load_import_manager

Tests for `LoadImportManager` — async load/import orchestration, result polling.

## Test cases

| Test | Description |
| ---- | ----------- |
| `poll_load_import_results_loaded_replaces_canvas` | `Loaded` result replaces document canvas and sets dimensions. |
| `poll_load_import_results_imported_replaces_canvas` | `Imported` result replaces document canvas with imported layers. |
| `poll_load_import_results_archive_imported_replaces_canvas` | `ArchiveImported` result replaces document canvas. |
| `poll_load_import_results_failed_appends_error` | `Failed` result pushes error and clears in-flight flags. |
| `poll_load_import_results_no_messages_is_noop` | No messages results in no-op. |
| `trigger_async_load_nonexistent_file_fails` | Loading a nonexistent file produces an error. |
