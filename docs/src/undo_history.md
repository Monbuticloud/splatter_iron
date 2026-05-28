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
