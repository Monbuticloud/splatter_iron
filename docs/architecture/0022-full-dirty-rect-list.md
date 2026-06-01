# ADR 22: Full Dirty-Rect List with Full-Blend Flag

- **Status:** Accepted
- **Date:** 2026-06-01

## Context

ADR 5 introduced a single `Option<DirtyRect>` for incremental rendering.
ADR 14 replaced it with `DirtyRectList` — a `Vec<DirtyRect>` with proximity
merging — but exposed a gap: structural canvas changes (layer add, delete,
reorder, opacity change) affect the entire composite. The rect-list approach
could track individual stroke regions, but had no way to signal "ignore all
rects, re-blend everything."

The initial workaround was to interpret an empty `DirtyRectList` as "full
re-blend needed" — but this conflated "no changes since last frame" with
"a full blend was requested." A stroke that happened to cover zero pixels
would accidentally trigger a full blend.

## Decision

Add a `needs_full_blend` boolean flag to `DirtyRectList`:

```rust
pub struct DirtyRectList {
    rects: Vec<DirtyRect>,
    needs_full_blend: bool,
}
```

### Methods

| Method | Behaviour |
|---|---|
| `new()` | Empty list, `needs_full_blend = false` |
| `request_full_blend()` | Set `needs_full_blend = true` |
| `add(rect)` | Push a dirty rect; **clears** `needs_full_blend` (incremental update available) |
| `merge_all()` | Merge all tracked rects into one bounding box |
| `take_all()` | Return `rects` and reset; **clears** `needs_full_blend` |
| `is_empty()` | `true` when no rects and `!needs_full_blend` |
| `needs_reblend()` | `true` when any rects exist or `needs_full_blend` |
| `clear()` | Remove all rects and reset flag |

### Integration

Structural operations call `request_full_blend()`:

```rust
// In document.rs — after layer add/delete/move/opacity change:
canvas.dirty_rect.request_full_blend();
```

The render frame checks `needs_reblend()` to decide whether to call
`blend_region`/`blend_layers` at all:

```rust
if !canvas.dirty_rect.needs_reblend() {
    return; // nothing changed since last upload
}
let rects = canvas.dirty_rect.take_all();
if rects.is_empty() {
    // needs_full_blend was true — blend entire canvas
    blend_layers(…);
} else {
    for rect in &rects {
        blend_region(…, rect);
    }
}
```

This eliminates the ambiguity of "empty list means full re-blend" and
makes the two states explicit: incremental rects vs. full-canvas re-blend.

### Constants (unchanged from ADR 14)

- `DIRTY_RECT_PROXIMITY: u32 = 16` — merge rects ≤16 px apart.
- `DIRTY_RECT_MAX_COUNT: usize = 8` — merge all if list exceeds 8 rects.

## Consequences

- **Positive:** `needs_full_blend` makes the two rendering modes explicit —
  the renderer no longer guesses intent from an empty rect list.
- **Positive:** A stroke of zero pixels (e.g. click without drag) no longer
  accidentally triggers a full-canvas re-blend — `add()` clears the flag.
- **Positive:** `needs_reblend()` provides a single early-exit check for the
  rendering pipeline — if nothing changed, skip all blend and upload work.
- **Positive:** Multiple structural changes between frames (e.g. delete layer
  then add layer) set the flag once and don't accumulate rects.
- **Negative:** Callers must remember to call `request_full_blend()` on
  structural operations — forgetting it leaves stale pixels until the next
  stroke triggers an incremental blend.
- **Negative:** The `add()` method clears `needs_full_blend` as a side effect,
  which could mask a concurrent structural change if a tool stroke arrives
  in the same frame.
