# undo_history

## `struct UndoHistory`

`UndoHistory` manages the undo/redo stack for the application. It wraps a `VecDeque<UndoRecord>` with a `redo_index` pointer that partitions the stack into undoable entries (below the index) and redoable entries (above). Brush-stroke deduplication is handled via per-pixel visited-stamp buffers that prevent the same pixel from being recorded multiple times during a single stroke frame.

The history is bounded at `MAX_STROKE_STACK = 1000` entries. When the stack exceeds this limit, the oldest entries are dropped from the front of the deque.

### The visited-stamp scheme

During a brush stroke, the same pixel may be touched across multiple frames (e.g., if the cursor moves slowly). Without deduplication, each frame would accumulate overlapping undo runs for the same pixels, causing wasted memory and incorrect undo (restoring an intermediate state rather than the original). The solution is a per-pixel stamp buffer:

- `visited` is a `Vec<u32>`, one entry per canvas pixel.
- Before recording a pixel, the stroke code checks if `visited[pixel_index] == visited_stamp`.
- If they match, the pixel has already been recorded in the current stroke — skip it.
- If they differ, record the pixel and set `visited[pixel_index] = visited_stamp`.
- `visited_stamp` is incremented once per stroke via [`next_stamp`], so each stroke starts fresh without needing to zero the entire buffer.

A parallel `drag_processed` buffer with its own `drag_stamp_value` provides the same deduplication within a single drag gesture (spanning multiple strokes), where the drag accumulator merges all frames' runs into one undo record.

### The drag accumulator

When the user drags the mouse across the canvas, each frame generates a new set of `RunSegment` values. Rather than pushing each frame as a separate undo record (which would make undo granular to individual frames), the drag accumulator merges all frames' runs and pushes a single `UndoRecord` when the drag ends (mouse up). This gives users a single undo step per drag gesture, which is the expected behavior in paint applications.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `stroke_stack` | `VecDeque<UndoRecord>` | Ordered history of undo records, most recent at the back |
| `redo_index` | `usize` | Partition point: entries >= this index are redoable |
| `visited` | `Vec<u32>` | Per-pixel stamp counters for intra-stroke deduplication |
| `visited_stamp` | `u32` | Global stamp counter, incremented per stroke |
| `drag_processed` | `Vec<u32>` | Per-pixel stamp counters for drag-gesture deduplication |
| `drag_stamp_value` | `u32` | Stamp counter for the current drag gesture |
| `drag_accumulator` | `Option<DragAccumulator>` | Accumulator for merging per-frame runs into one record |

### Invariants

- `redo_index` must always be `<= stroke_stack.len()`. It is reset to 0 on every `push_undo`.
- `visited_stamp` starts at 1 (0 is the "unvisited" sentinel). When it wraps past `u32::MAX`, the `visited` buffer is zeroed and the stamp resets to 1.
- `drag_stamp_value` follows the same wrap-and-reset pattern for `drag_processed`.
- The `drag_accumulator` is `None` outside of an active drag gesture (initialized by `init_drag_accumulator`, consumed by `finalize_drag_accumulator`).

## `impl UndoHistory::new(pixel_count)`

Constructs an empty undo history with pre-allocated visited-stamp buffers sized to the canvas pixel count. This is called once when the application starts or when a new document is created.

### Allocation strategy

Both `visited` and `drag_processed` are allocated as `vec![0u32; pixel_count]` in a single call each. For a typical 4K canvas (3840×2160 ≈ 8.3M pixels), each buffer consumes 32 MB (8.3M × 4 bytes), totaling 64 MB for both buffers. This memory is allocated upfront and reused across the document's lifetime; it is only reallocated if the canvas grows via [`resize_visited`].

### Parameters

| Parameter | Type | Purpose |
|-----------|------|---------|
| `pixel_count` | `usize` | Number of pixels in the canvas, determines visited buffer sizes |

### Initial state

| Field | Initial value |
|-------|---------------|
| `stroke_stack` | Empty `VecDeque` |
| `redo_index` | `0` |
| `visited` | `vec![0u32; pixel_count]` |
| `visited_stamp` | `1` (0 is the unvisited sentinel) |
| `drag_processed` | `vec![0u32; pixel_count]` |
| `drag_stamp_value` | `1` |
| `drag_accumulator` | `None` |

## `impl UndoHistory::push_undo(record)`

Pushes a new undo record onto the history stack and invalidates any existing redo history. This is called after each completed stroke or finalized drag gesture.

### Operations

1. **Truncate redo**: `stroke_stack.truncate(stroke_stack.len() - redo_index)` removes all redoable entries from the back of the deque. This implements the standard undo-behavior: a new action after an undo discards the stale redo history.
2. **Push record**: `stroke_stack.push_back(record)` appends the new record as the most recent entry.
3. **Enforce limit**: If the stack exceeds `MAX_STROKE_STACK` (1000), the oldest entry is popped from the front via `pop_front()`.
4. **Reset redo_index**: Set to 0, indicating no redoable entries remain.

### Parameters

| Parameter | Type | Purpose |
|-----------|------|---------|
| `record` | `UndoRecord` | The record to push onto the history stack |

### Stack-limit behavior

When the limit is hit, the oldest stroke is silently dropped. This means the user loses the ability to undo the earliest strokes in the session. The limit of 1000 is generous enough for most painting sessions — even at 10 strokes per second, a user would need 100 seconds of continuous drawing to hit the cap.

## `impl UndoHistory::next_stamp()`

Increments the global `visited_stamp` counter and returns the new value. Each call effectively starts a new stroke for deduplication purposes: subsequent pixel writes will record their before-pixels in the undo record because no pixel's visited stamp matches the new value.

### Overflow handling

When `visited_stamp` wraps past `u32::MAX` back to 0, the visited buffer is zeroed in O(N) time and the stamp resets to 1. This is necessary because stamp 0 is the global "unvisited" sentinel used during buffer initialization and resize. Without this reset, pixels never visited in the current stamp epoch would still carry stale stamps from the prior epoch, causing false deduplication hits.

### Returns

A `u32` stamp value guaranteed to differ from all previous stamps (modulo overflow). Callers use this to initialize `visited_stamp` before emitting a stroke's pixels.

## `impl UndoHistory::resize_visited(pixel_count)`

Grows the visited-stamp and drag-processed buffers to accommodate a new canvas size. Called when the canvas dimensions change (e.g., canvas resize, new document) to ensure the undo system's internal buffers match the pixel array length.

### Grow-only policy

Buffers are only resized if the requested `pixel_count` exceeds the current buffer length. Shrinking is not performed — if the canvas shrinks, the extra entries at the end of the buffer are simply unused. This avoids unnecessary deallocation-reallocation cycles when the user resizes back and forth.

### Post-resize state

| Field | After `resize_visited` |
|-------|------------------------|
| `visited` | New `vec![0u32; pixel_count]` if grown; unchanged otherwise |
| `drag_processed` | New `vec![0u32; pixel_count]` if grown; unchanged otherwise |
| `visited_stamp` | Reset to `1` unconditionally |
| `drag_stamp_value` | Reset to `1` unconditionally |

### Parameters

| Parameter | Type | Purpose |
|-----------|------|---------|
| `pixel_count` | `usize` | Required number of entries in the visited buffers |

### Why unconditional stamp reset

Even when buffers don't need growing (canvas shrank), the stamp is reset to 1. This is a correctness measure: stall stamps from the prior canvas are invalid for the new pixel geometry, and reusing them could cause false deduplication at overlapping index ranges.

## `impl UndoHistory::advance_drag_stamp()`

Increments the drag-scoped `drag_stamp_value` without consuming it as a return value. Each call marks a new frame within the current drag gesture, allowing per-frame deduplication via the `drag_processed` buffer.

### Relationship to `next_stamp`

`next_stamp` operates at the stroke level — it is called once per brush stroke (a single mouse-down-to-mouse-up sequence). `advance_drag_stamp` operates at the frame level within a drag — each frame of the drag receives a fresh stamp so that pixels touched in one frame are not re-recorded if the stroke revisits them in a later frame. The drag accumulator collects all frames' runs and merges them into one record; the frame-level stamps prevent duplicate runs for overlapping pixels across frames.

### Overflow handling

Same pattern as `next_stamp`: when `drag_stamp_value` wraps past `u32::MAX`, the `drag_processed` buffer is zeroed in O(N) time and the stamp resets to 1.

## `impl UndoHistory::init_drag_accumulator(layer_index, width, color_after, is_alpha_overlay)`

Begins accumulating undo data for a new drag gesture. Called on mouse-down (or pen-down/touch-start) before any pixels are drawn. Sets the layer and color metadata that will be shared across all frames of the drag.

### Drag-accumulation lifecycle

```
mouse_down()
  → init_drag_accumulator(layer, width, color, alpha)
  → frame_1():
      draw_something()
      → extend_drag_accumulator(frame_1_runs)
  → frame_2():
      draw_more()
      → extend_drag_accumulator(frame_2_runs)
  → ...
  → mouse_up()
      → finalize_drag_accumulator()  // pushes one UndoRecord
```

### Parameters

| Parameter | Type | Purpose |
|-----------|------|---------|
| `layer_index` | `usize` | Target layer for this drag gesture |
| `width` | `u32` | Canvas width (stored for potential row-stride computations) |
| `color_after` | `Color32` | Color applied by the drag stroke |
| `is_alpha_overlay` | `bool` | Whether the drag uses alpha-overlay blending |

### Internal state

After calling `init_drag_accumulator`, the `drag_accumulator` field is `Some(DragAccumulator { runs: vec![], layer_index, width, color_after, is_alpha_overlay })`. The runs vector starts empty and is populated by subsequent calls to `extend_drag_accumulator`.

## `impl UndoHistory::extend_drag_accumulator(runs)`

Accumulates a frame's worth of run segments into the current drag accumulator. Called at the end of each frame during an active drag gesture, after the tool has drawn its pixels and collected the affected runs.

### Prepending behavior

New runs are **prepended** before previously accumulated runs (not appended). That is:

```rust
let mut combined = new_runs;
combined.append(&mut accumulator.runs);
accumulator.runs = combined;
```

This means the most recent frame's runs appear first in the final `UndoRecord`. For `undo_apply`, which processes runs in order, this ensures the most recent pixels are restored first, which is correct for overlapping non-alpha paint (each step walks back through intermediate states to the original). For alpha overlay, runs are guaranteed disjoint by `drag_processed` deduplication, so prepending has no correctness impact.

### Parameters

| Parameter | Type | Purpose |
|-----------|------|---------|
| `runs` | `Vec<RunSegment>` | Run segments captured during the current drag frame |

### Safety

No-op if no drag accumulator is active (i.e., `init_drag_accumulator` was not called, or `finalize_drag_accumulator` already consumed it). The guard is a simple `if let Some(...)`.

## `impl UndoHistory::finalize_drag_accumulator()`

Completes a drag gesture by consuming the accumulated `DragAccumulator` and pushing its contents as a single `UndoRecord` onto the history stack. Called on mouse-up (or pen-up/touch-end).

### Algorithm

1. Take ownership of the accumulator via `self.drag_accumulator.take()`, leaving `None` in its place.
2. Construct `UndoRecord::Run` from the accumulator's stored metadata and accumulated runs.
3. Push the record via `self.push_undo(record)`, which handles truncation, stack-limit enforcement, and redo-index reset.

### Idempotency

Calling `finalize_drag_accumulator` when no drag is in progress is a safe no-op. The `take()` returns `None`, the `if let Some(...)` guard fires, and nothing happens. This simplifies cleanup code — the drag-end handler can unconditionally call `finalize_drag_accumulator` without tracking whether a drag was actually in progress.

## `impl UndoHistory::clear()`

Resets the undo history to its initial empty state. Drains all entries from `stroke_stack` and sets `redo_index` to 0. The visited-stamp buffers (`visited`, `drag_processed`) and stamp counters (`visited_stamp`, `drag_stamp_value`) are **not** reset — they remain valid for the current canvas.

### When to call

- Loading a new document (the new document has no history).
- Creating a new canvas (same reason).
- After a destructive operation that invalidates the undo history (e.g., canvas resize, layer deletion).

### What persists

After `clear()`, the allocation of the visited buffers is preserved. No reallocation occurs. This is intentional: `clear()` is often followed by more drawing, and preserving the allocations avoids unnecessary work.

## `impl UndoHistory::can_undo()`

Returns `true` if there is at least one stroke in the history stack that can be undone.

### Logic

```rust
self.redo_index < self.stroke_stack.len()
```

Entries at indices `[0, stroke_stack.len() - redo_index)` are undoable. As undo steps are applied, `redo_index` grows toward `stroke_stack.len()`. When `redo_index == stroke_stack.len()`, all entries have been undone and there is nothing left to undo.

## `impl UndoHistory::can_redo()`

Returns `true` if there is at least one previously undone stroke that can be reapplied.

### Logic

```rust
self.redo_index > 0
```

Entries at indices `[stroke_stack.len() - redo_index, stroke_stack.len())` are redoable. When `redo_index` is 0, no entries have been undone and there is nothing to redo. A non-zero `redo_index` means entries were undone and are available for redo.

### State transitions

| Action | `redo_index` | `can_undo` | `can_redo` |
|--------|-------------|------------|------------|
| Initial (empty) | 0 | false | false |
| After push | 0 | true | false |
| After 1 undo | 1 | true (if >1 entries) | true |
| After all undone | N (= len) | false | true |
| After 1 redo | N-1 | true | true (if >0) |
| After new push | 0 | true | false |
