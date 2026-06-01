# tests::brush_common

## Test strategy

Tests for `apply_visited_runs` in `brush_common`, covering empty-visit, full-visit, alpha-overlay blend, and processed-pixel skip behavior.

## `no_visited_pixels_returns_empty`

Verifies that `apply_visited_runs` returns an empty run list when no pixels are marked visited.

## `all_visited_produces_runs`

Confirms that when all pixels are visited, `apply_visited_runs` produces one run per row and writes the brush color to every pixel.

## `alpha_overlay_blends`

Ensures that `apply_visited_runs` in alpha-overlay mode blends the overlay color with existing pixel values rather than replacing them.

## `alpha_overlay_skips_processed`

Verifies that `apply_visited_runs` skips pixels already marked as processed in the drag buffer during alpha-overlay mode, returning an empty run list when none remain unprocessed.
