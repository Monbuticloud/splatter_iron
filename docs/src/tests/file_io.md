# tests::file_io

Tests for `FileIO` — the mpsc-channel plumbing that connects UI events to asynchronous file dialogs.

## Test strategy

- Synthetic channel pairs are injected via `test_file_io` to avoid spawning real OS dialogs.
- `poll_save_results` is tested for autosave, manual save, failure, and no-message cases.
- `poll_dialog_results` is tested for pending-save and mismatched-pending skip.
- Async save is tested by `trigger_async_save` with a real temp directory and a short sleep.

## `test_file_io` (helper)

Constructs a `FileIO` with synthetic `mpsc` channels for dialog and save results.

## `poll_save_results_autosave_clears_dirty`

Receiving `SaveResult::Autosave` clears `dirty_since_last_autosave`.

## `poll_save_results_manual_save_sets_path`

Receiving `SaveResult::ManualSave(path)` sets the document's `savefile_path` and requests a re-render.

## `poll_save_results_failed_appends_error`

Receiving `SaveResult::Failed(msg)` appends the message to the error list.

## `poll_save_results_no_messages_is_noop`

When no messages are on the channel, `poll_save_results` is a no-op.

## `poll_dialog_results_save_triggers_async_save`

Receiving `DialogResult::Picked` with a pending `Save` action spawns an async save and consumes the pending action.

## `poll_dialog_results_mismatched_pending_skips`

A dialog result arrives but the pending action does not match — the message is consumed but skipped without error.

## `save_to_current_path_empty_path_noop`

Calling `save_to_current_path` with an empty save path does nothing.

## `trigger_async_save_writes_file`

`trigger_async_save` writes a valid `.splattercanvas` file to disk, verified by checking file existence after a 100 ms sleep.
