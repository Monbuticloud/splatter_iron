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
