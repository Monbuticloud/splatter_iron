# tests::undo_history

Tests for `UndoHistory` — the undo/redo stack, visited-stamp deduplication, drag-accumulator lifecycle, and multi-step operations.

## Test strategy

- State-machine tests: exercise `can_undo` / `can_redo` through push → undo → redo cycles.
- Multi-step: `undo_step(n)` and `redo_step(n)` with `n > 1`.
- Edge cases: stamp-counter wrapping past `u32::MAX`, clearing the history, truncating redo on new push.

## `new_history_has_no_undo_no_redo`

A freshly created `UndoHistory` has neither undo nor redo available.

## `push_undo_makes_undo_available`

After pushing one record, `can_undo()` returns `true` and `can_redo()` returns `false`.

## `undo_step_applies_and_tracks_redo`

Undoing one step restores original pixels and makes redo available.

## `redo_step_reapplies`

After undo, redo reapplies the stroke and consumes the redo entry.

## `push_undo_truncates_redo_stack`

Pushing a new record after undoing clears the redo stack (standard undo-model behaviour).

## `undo_redo_multi_step`

Undoing two steps at once restores the pre-stroke state; redoing two steps reapplies both strokes in order.

## `clear_resets_history`

`clear()` removes all undo and redo entries.

## `next_stamp_increments`

Consecutive `next_stamp()` calls return incrementing values (mod `u32::MAX`).

## `stamp_wrapping_overflow_resets`

When the internal stamp counter wraps from `u32::MAX`, it resets to 1 and clears all visited stamps (prevents stale-dedup collisions).

## `resize_visited_grows_buffer`

`resize_visited(250)` grows both `visited` and `drag_processed` buffers from 100 to 250 entries and resets stamps to 1.

## `resize_visited_does_not_shrink`

`resize_visited(50)` on a 100-entry buffer is a no-op — the buffer does not shrink.

## `advance_drag_stamp_increments`

`advance_drag_stamp` increments `drag_stamp_value` by 1.

## `advance_drag_stamp_wrapping_resets`

When `drag_stamp_value` wraps past `u32::MAX`, it resets to 1 and clears the `drag_processed` buffer.

## `drag_accumulator_full_lifecycle`

A full drag lifecycle (init → extend across two frames → finalize) produces a single undo record that can be undone to restore the original state.

## `undo_step_clamps_to_available`

Requesting 10 undo steps when only 1 is available does not panic and fully consumes the available entry.

## `redo_step_clamps_to_available`

Requesting 10 redo steps when only 1 is available does not panic and fully consumes the available entry.

## `max_stack_eviction_pops_oldest`

Pushing 1001 records (one past `MAX_STROKE_STACK = 1000`) evicts the oldest; the stack remains at 1000 entries.

## `extend_drag_without_init_noop`

Calling extend_drag_accumulator without init is a no-op.

## `extend_drag_without_init_with_runs_noop`

Extending drag with runs but without init is a no-op.

## `finalize_drag_without_init_noop`

Finalizing drag without init is a no-op (returns None).
