# ADR 14: Dirty-Rect List with Proximity Merging

- **Status:** Accepted
- **Date:** 2026-05-28

## Context

ADR-0005 introduced a single `DirtyRect` bounding box to track the modified
region for incremental rendering. This gave a ~5–10× reduction in per-frame
blend work for typical strokes. However, the single-rect approach has a
coarse approximation problem: two distant stroke endpoints union into one
large rect that covers many unchanged pixels between them. For example,
painting at (100, 100) and then at (1000, 100) creates a 900-pixel-wide
dirty rect — but only ~200 columns actually changed (two brush stamps).

Additionally, each tool function updates `dirty_rect` via `union()`, which
requires careful branching for the `None`/`Some(rect)` Option pattern.

## Decision

Replace `Option<DirtyRect>` with a `DirtyRectList` that holds a `Vec<DirtyRect>`
and tracks each dirty region individually:

```rust
pub struct DirtyRectList {
    rects: Vec<DirtyRect>,
}
```

### Merge strategy

When a new rect is added (`DirtyRectList::add`):

1. Scan existing rects for overlap or proximity (≤16 px gap).
2. If a match is found, absorb the existing rect into the new one via
   `union()` and remove it (swap-remove for O(1) removal).
3. Push the (possibly merged) rect onto the list.
4. If the list exceeds 8 rects, merge all into a single bounding box
   (bounded worst-case behaviour).

This approximates a "dirty-region list" without the complexity of spatial
partitioning (quadtrees, grid buckets). The proximity threshold of 16 px
and max-count of 8 were chosen empirically — brush strokes tend to produce
small, localised rects that either overlap or are far apart.

### Integration

`Canvas` stores `dirty_rect: DirtyRectList` instead of
`dirty_rect: Option<DirtyRect>`:

```
// Before:
canvas.dirty_rect = match canvas.dirty_rect {
    Some(rect) => Some(rect.union(&new_rect)),
    None => Some(new_rect),
};

// After:
canvas.dirty_rect.add(new_rect);
```

Tool functions call `canvas.dirty_rect.add(DirtyRect::new(...))` directly
instead of the `match` pattern.

### Rendering

`Document::blend_to_output` iterates over all rects from
`dirty_rect.take_all()`:

```rust
let rects = self.canvas.dirty_rect.take_all();
for rect in &rects {
    pixel::blend_region(…, rect.min_x, rect.min_y, rect.max_x, rect.max_y);
}
```

For the wgpu GPU path, each rect produces its own `write_texture` call with
the corresponding sub-region offset. The Glow fallback always uploads the
full texture (unchanged).

## Consequences

- **Positive:** Eliminates the "single huge rect between two distant points"
  problem — non-overlapping stroke regions stay as separate rects, reducing
  per-frame blend area by ~30–60% in multi-stroke scenarios.
- **Positive:** Removes the `Option<DirtyRect>` branching pattern from all
  tool functions — `add()` handles empty/inverted rects internally.
- **Positive:** `merge_all()` provides a clean escape hatch when the rect
  count grows too large, bounding worst-case uploads.
- **Negative:** per-frame rendering must iterate multiple rects instead of
  a single optional one — extra loop overhead (negligible for ≤8 rects).
- **Negative:** Proximity merging (16 px) may merge rects that are visually
  separate but close together, creating a slightly larger dirty region than
  strictly necessary.
- **Negative:** The wgpu GPU path issues multiple `write_texture` calls per
  frame (one per rect) instead of one — driver overhead may offset savings
  for very small rects.
