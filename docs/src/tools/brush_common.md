# brush_common

Shared brush utilities for visited-pixel run capture.

Provides [`apply_visited_runs`] which is used by `square_brush`,
`circle_brush`, and `stamp_brush` to collect before-pixel data,
write new colour values, and produce compressed [`RunSegment`]s
for undo records.

## `apply_visited_runs`

```rust
pub fn apply_visited_runs(
    pixels: &mut [Color32],
    dirty_rect: &DirtyRect,
    width: usize,
    visited: &[u32],
    stamp: u32,
    color: Color32,
    alpha_overlay: bool,
    drag_processed: &mut [u32],
    drag_stamp_value: u32,
) -> Vec<RunSegment>
```

Apply color to all visited pixels within a dirty region, capture
before-pixels, and return compressed run segments for undo.

Iterates the dirty rect row by row. For each pixel marked with the
current `stamp` value (and not already drag-processed in alpha-overlay
mode), captures the old color, writes the new color (blend or replace),
and assembles contiguous runs.

### Parameters

| Parameter          | Type             | Purpose                                               |
| ------------------ | ---------------- | ----------------------------------------------------- |
| `pixels`           | `&mut [Color32]` | Mutable layer pixels                                  |
| `dirty_rect`       | `&DirtyRect`     | Bounding box of the stamped region                    |
| `width`            | `usize`          | Canvas width in pixels                                |
| `visited`          | `&[u32]`         | Stamp buffer marking which pixels this stroke touches |
| `stamp`            | `u32`            | The current stamp value to match against `visited`    |
| `color`            | `Color32`        | Colour to apply (premultiplied-alpha)                 |
| `alpha_overlay`    | `bool`           | If true, alpha-blend instead of overwriting           |
| `drag_processed`   | `&mut [u32]`     | Per-pixel drag-stamp buffer (for alpha-overlay dedup) |
| `drag_stamp_value` | `u32`            | The current drag-stamp value                          |

### Returns

A vector of `RunSegment` suitable for embedding in an `UndoRecord::Run`.

### Panics

Panics if `pixels` is too small to cover the dirty rect at the given
`width`, or if `visited` / `drag_processed` are too small.
