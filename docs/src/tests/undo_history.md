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
