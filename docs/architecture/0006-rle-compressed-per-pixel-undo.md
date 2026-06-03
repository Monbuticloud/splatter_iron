# ADR 6: RLE-Compressed Per-Pixel Undo/Redo

- **Status:** Accepted
- **Date:** 2026-05-18
- **Commits:** `d17c90e`, `58f1699`
- **Superseded-By:** ADR-0024 (flat before-pixels buffer + zstd compression partially replaced the per-segment storage model)

## Context

Brush strokes modify arbitrary sets of pixels. To support undo/redo, the
application must record enough information to revert each stroke. The initial
implementation stored each modified pixel individually as a `StrokePixel`
(three `u32` fields: index, before color, after color). For a typical brush
stroke of radius 100 covering ~31,000 pixels (`πr²`), this meant:

- 31,000 × 3 × 4 = 372 KB per stroke
- `Vec<StrokePixel>` allocation per stroke with per-pixel `push()`
- Linear scan of the full pixel list during undo/redo

For a maximum undo stack of 1,000 strokes, worst-case memory usage could reach
~372 MB for large brushes.

## Decision

Replace `Stroke`/`StrokePixel` with an `UndoRecord::Run` variant that stores
contiguous runs of pixels with run-length compression:

### `UndoRecord` enum

```rust
pub enum UndoRecord {
    Run {
        layer_index: usize,
        color_after: Color32,
        runs: Vec<RunSegment>,
        is_alpha_overlay: bool,
    },
}
```

### Run-length compression

```rust
pub enum BeforePixels {
    All(Color32),   // Entire run was a single color (compact)
    Many(Vec<Color32>), // Run had non-uniform colors (full vec)
}

pub fn compress_run(pixels: Vec<Color32>) -> (BeforePixels, u32) {
    if length < 8 { return Many(pixels); }        // too short to matter
    if all_same(pixels) { return All(first); }    // uniform → 1 Color32
    else { return Many(pixels); }                 // non-uniform → full vec
}
```

- Runs shorter than 8 pixels are always stored as `Many` — the overhead of
  checking uniformity doesn't pay off.
- Uniform runs longer than 8 pixels store a single `Color32` instead of N.
- `redo_apply` fills the run range with `color_after` (or alpha-blends for
  overlay strokes).
- `undo_apply` restores either the single uniform color or copies the full vec.

The earlier `StrokePixel` approach (per-pixel index+before+after) was removed
in commit `72ad30b`, leaving only the `Run` variant.

## Consequences

- **Positive:** A uniform brush stroke (typical use case) stores ~32 bytes per
  run instead of 12 bytes per pixel — a ~375× compression ratio for large
  uniform spans.
- **Positive:** `undo_apply`/`redo_apply` use `fill()` or `copy_from_slice()`
  instead of per-pixel writes — faster memory access patterns.
- **Positive:** Maximum undo stack of 1,000 strokes fits within reasonable
  memory (~tens of MB vs hundreds of MB).
- **Negative:** Non-uniform runs (e.g., painting over a multi-colored
  background) fall back to `Many(Vec<Color32>)` with no compression.
- **Negative:** The `compress_run` check is O(n) and allocates a temporary
  `Vec` before compression — a wasted allocation for short or non-uniform runs.
- **Negative:** The `is_alpha_overlay` flag adds complexity to both apply and
  undo paths (alpha-blend vs fill).
