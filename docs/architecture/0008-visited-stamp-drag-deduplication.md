# ADR 8: Visited-Stamp Drag Deduplication

- **Status:** Accepted
- **Date:** 2026-05-18
- **Commits:** `5e629e3`, `a4f21bd`, `0644831`

## Context

When a user drags the mouse to draw a brush stroke, the application receives
mouse-move events each frame. For each segment of the drag, the brush-line
algorithm (Bresenham interpolation) produces a set of pixel indices that lie
under the brush between the previous and current cursor positions.

The naive approach was to collect all touched indices, sort them, and deduplicate
via `sort_unstable() + dedup()`. This has two problems:

1. **O(n log n) sorting** per frame — sorting tens of thousands of `u32` values
   is expensive and gets worse with larger brushes.
2. **Overlap between frames** — pixels touched in frame N can be re-touched in
   frame N+1 when the brush moves slowly. Without deduplication, the same pixel
   is alpha-blended multiple times, darkening it.

## Decision

Use a **visited-stamp buffer** — a `Vec<u32>` the size of the canvas where each
pixel stores a stamp value. A stroke sets `visited[pixel] = stamp` for every
touched pixel. Later frames check `visited[pixel] != stamp` to skip already
processed pixels.

```rust
// Per-pixel stamp counter, one per canvas pixel
pub visited: Vec<u32>,       // scratch buffer for brush-line dedup
pub visited_stamp: u32,       // incremented per stroke

// Drag-scoped overlay dedup (alpha mode)
pub drag_processed: Vec<u32>, // per-pixel drag stamp
pub drag_stamp_value: u32,    // incremented per drag gesture
```

### Algorithm

```
next_stamp → stamp++
for each drag segment:
    for each pixel under brush:
        if visited[pixel] != stamp:   // never seen this stroke
            visited[pixel] = stamp
            capture before-pixel
            apply color/blend
    if alpha_overlay:
        mark drag_processed[pixel] = drag_stamp_value
```

Stamp values wrap around at `u32::MAX` and reset the buffer to zero on
overflow. Drag-scoped stamps (`drag_processed`) are advanced per gesture via
`advance_drag_stamp()` and prevent re-blending the same pixel within a single
drag when using alpha overlay mode.

### Why not a `HashSet<u32>`?

- `HashSet` has per-entry allocation and hash overhead.
- `Vec<u32>` with stamp check is O(1) per pixel with no heap allocation (the
  buffer is pre-allocated at canvas size).
- Stamp buffers are allocated once at canvas creation and resized when the
  canvas changes.

## Consequences

- **Positive:** O(1) per-pixel visit check via array lookup. Eliminated
  `sort_unstable() + dedup()` — now O(n) instead of O(n log n).
- **Positive:** No per-stroke allocation for the visited buffer; allocated once
  at `UndoHistory::new(pixel_count)`.
- **Positive:** The drag stamp (`drag_processed`) correctly handles alpha
  overlay mode where the same pixel might be visited by overlapping brush-line
  segments in the same frame — without it, alpha would accumulate incorrectly.
- **Negative:** Stamp buffer size = canvas pixels × 4 bytes × 2 buffers. For
  2000×1500: ~24 MB. Doubles canvas memory footprint.
- **Negative:** Stamp overflow at `u32::MAX` requires an O(n) buffer reset —
  unlikely in practice (at 60 fps, ~2 years of continuous strokes).
- **Negative:** Two stamp buffers (visited + drag_processed) with separate
  stamp values adds complexity to the brush-line functions.
