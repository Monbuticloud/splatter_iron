# tests::brush_common

## Test strategy

Tests for `apply_visited_runs` in `brush_common`, covering empty-visit, full-visit, alpha-overlay blend, and processed-pixel skip behavior.

## `no_visited_pixels_returns_empty`

Verifies that `apply_visited_runs` returns an empty run list when no pixels are marked visited.
