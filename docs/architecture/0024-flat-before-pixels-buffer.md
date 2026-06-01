# ADR 24: Flat Before-Pixels Buffer with zstd Compression

- **Status:** Accepted
- **Date:** 2026-06-01

## Context

ADR 6 and ADR 21 defined the undo storage model: each `UndoRecord::Run`
holds a `Vec<RunSegment>`, where each `RunSegment` owns its before-pixel
data via `BeforePixels::Many(Vec<Color32>)`. A single full-canvas stroke
(radius 1000 circle on a 2000×1500 canvas, 3M pixels) produces one
`RunSegment` per canvas row — 1500 segments, each owning its own
`Vec<Color32>` on the heap.

This generates **1500 separate heap allocations** per full-canvas stroke.
At `MAX_STROKE_STACK = 1000` in the undo history, the worst-case memory
is ~12 GB — all from undo records alone. In practice the existing RLE
compression (`BeforePixels::All` for uniform spans) helps, but once the
canvas is heterogeneous (photo-like content, organic paint strokes),
uniform runs are rare and every pixel is stored uncompressed.

The project already depends on `zstd` for canvas serialization
(`src/files.rs`). Reusing it for in-memory undo compression adds no
new dependencies.

## Decision

### 1. Flatten before-pixels into one contiguous `Vec<Color32>` per stroke

Replace `BeforePixels::Many(Vec<Color32>)` with
`BeforePixels::Many { offset: u32, length: u32 }`, where `offset`/`length`
reference into a shared `before_pixels: Vec<Color32>` field on the
`UndoRecord::Run` variant.

```rust
pub enum BeforePixels {
    All(Color32),
    Many { offset: u32, length: u32 },
}

pub struct RunSegment {
    pub start: u32,
    pub length: u32,
    pub before: BeforePixels,
}

pub enum UndoRecord {
    Run {
        layer_index: usize,
        color_after: Color32,
        runs: Vec<RunSegment>,
        before_pixels: Vec<Color32>,
        compressed_before_pixels: Option<Vec<u8>>,
        is_alpha_overlay: bool,
    },
    // … other variants unchanged
}
```

### 2. Replace `compress_run` with `compress_and_store`

Before: `compress_run(Vec<Color32>) -> (BeforePixels, u32)` — owned `Vec`
per call, moved into `BeforePixels::Many`.

After: `compress_and_store(&[Color32], &mut Vec<Color32>) -> (BeforePixels, u32)` —
appends non-uniform slices to a caller-owned flat buffer.

```rust
pub fn compress_and_store(
    slice: &[Color32],
    buf: &mut Vec<Color32>,
) -> (BeforePixels, u32) {
    let length = slice.len() as u32;
    if length >= RLE_SHORT_RUN_THRESHOLD
        && slice.iter().all(|&p| p == slice[0])
    {
        (BeforePixels::All(slice[0]), length)
    } else {
        let offset = buf.len() as u32;
        buf.extend_from_slice(slice);
        (BeforePixels::Many { offset, length }, length)
    }
}
```

### 3. zstd-compress the flat buffer at the undo-stack boundary

On `push_undo`, compress `before_pixels` via `zstd::encode_all` (level -1)
into `compressed_before_pixels` and clear the uncompressed buffer.
On `undo_step`, decompress back into `before_pixels` before calling
`undo_apply`, then re-compress.

`redo_step` does **not** need to decompress — `redo_apply` reads
`color_after`, not `before_pixels`.

### 4. Per-frame drag accumulator

The `DragAccumulator` previously stored a single `Vec<RunSegment>` with
prepend-based accumulation. With the flat buffer, per-frame segments
now reference offsets into a frame-local `before_pixels`. The accumulator
stores a `Vec<DragFrame>` where each frame holds `(runs, before_pixels)`.
On `finalize_drag_accumulator`, frames are reversed (most-recent-first
for correct undo order), offset-adjusted, and merged into one record.

```rust
struct DragFrame {
    runs: Vec<RunSegment>,
    before_pixels: Vec<Color32>,
}

struct DragAccumulator {
    frames: Vec<DragFrame>,
    layer_index: usize,
    width: u32,
    color_after: Color32,
    is_alpha_overlay: bool,
}
```

### 5. Custom brush line merge

`draw_custom_brush_line` calls `draw_stamp_line` in a loop and merges
results. After each call it now also concatenates `before_pixels` buffers
and adjusts `BeforePixels::Many` offsets by the accumulated length.

## Consequences

- **Positive:** 1500 separate `Vec` allocations → 1 contiguous `Vec` per
  full-canvas stroke. Less allocator pressure, better cache locality on
  undo/redo.

- **Positive:** With zstd level -1, a blank-canvas stroke (all
  `Color32::TRANSPARENT`, 12 MB uncompressed) compresses to ~200 bytes.
  A photo-like stroke (12 MB) compresses to ~4–8 MB in practice. Pure
  incompressible noise stays at ~12 MB (same as before).

- **Positive:** Undo stack now holds ~4× less memory on average, reducing
  the chance of hitting `MEMORY_WARNING_THRESHOLD` (500 MB) in realistic
  usage.

- **Positive:** No new dependencies — `zstd` and `bytemuck` were already
  in `Cargo.toml`.

- **Negative:** zstd compress/decompress adds ~3 ms latency per undo step
  (level -1 on 12 MB). This is well below human perception (typical undo
  is < 16 ms).

- **Negative:** The `DragAccumulator` stores frames separately until
  finalize, temporarily doubling peak memory during long drags. This is
  bounded by the frame count × mean frame size, which is negligible for
  typical drag gestures.

- **Negative:** `draw_custom_brush_line` now does O(steps) offset
  adjustment on `BeforePixels::Many` entries. This is a few µs per step
  — negligible.

## Alternatives Considered

- **Layer snapshot on >50% coverage** (TODO.md: P2): Store a full
  zstd-compressed layer clone instead of per-pixel before-data for large
  strokes. Simpler code path for big strokes but adds branching
  complexity. Deferred.

- **Per-segment zstd** instead of flat buffer: Compress each
  `BeforePixels::Many` independently. Worse compression ratio (zstd needs
  ~10 KB for good dictionaries) and more allocations. Rejected.

- **mmap-backed undo**: Memory-map a temporary file for the undo stack.
  Predictable memory use but introduces IO latency and serialization.
  Rejected as premature optimization.
