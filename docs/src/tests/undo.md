# tests::undo

Tests for per-pixel undo/redo record compression and application: `compress_run`, `undo_apply`, `redo_apply`.

## Test strategy

- `compress_run` is tested at and around the `RLE_SHORT_RUN_THRESHOLD` (8) to confirm that short runs produce `BeforePixels::Many` and long uniform runs produce `BeforePixels::All`.
- Full round-trips (`undo_apply ∘ redo_apply`) verify that stroke pixels are correctly preserved and restored.
- Stacking behaviour is tested with multiple consecutive strokes.

## `compress_run_short_returns_many`

A 4-pixel run stays below the threshold → `BeforePixels::Many`.

## `compress_run_uniform_long_returns_all`

A 20-pixel uniform run exceeds the threshold → `BeforePixels::All` with the uniform color.

## `compress_run_mixed_long_returns_many`

A 20-pixel non-uniform run (alternating red/blue) exceeds the threshold but cannot be stored as `All` → `BeforePixels::Many`.

## `compress_run_threshold_boundary`

A run of exactly 8 pixels (at the threshold) is still classified as short → `Many`.

## `compress_run_just_above_threshold`

A uniform run of 9 pixels (one above threshold) is classified as long → `All`.

## `undo_apply_restores_before_pixels`

After drawing a red square, `undo_apply` restores the original white pixel values.

## `redo_apply_reapplies_color`

After undo, `redo_apply` reapplies the stroke color.

## `undo_redo_full_roundtrip`

A full undo → redo → undo cycle restores the original state each time.

## `undo_record_is_runs_variant`

`draw_square` produces an `UndoRecord::Run` variant.

## `empty_square_produces_empty_runs`

A zero-area square produces an undo record whose runs list is empty.

## `compress_run_empty_returns_many`

An empty pixel vec returns length 0 and `BeforePixels::Many`.

## `redo_apply_alpha_overlay_blends`

`redo_apply` on an alpha-overlay stroke blends the colour over the restored background, matching the original blended result (undo→redo is lossless for alpha-overlay strokes).

## `undo_apply_before_pixels_all_restores`

A uniform full-canvas square compresses each run as `BeforePixels::All`. `undo_apply` restores all pixels to the original white using the `fill` path.

## `multiple_undos_stack`

Two consecutive strokes can be undone in reverse order and redone in original order, composing correctly.
