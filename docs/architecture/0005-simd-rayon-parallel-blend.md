# ADR 5: SIMD + Rayon Parallel Pixel Blending

- **Status:** Accepted
- **Date:** 2026-05-17
- **Commits:** `049e6e2`, `773ee98`

## Context

Canvas compositing (blending N layers into a single RGBA output) is the
single most performance-critical operation in SplatterIron. It is called
every frame during brush strokes — a fast path for the full canvas
(2000×1500 = 3M pixels × N layers) and a partial path for dirty-rect
(usually ~1000–100,000 pixels).

A scalar per-pixel blend (one `alpha_blend` call per layer per pixel) is
too slow for interactive repainting:
- 3M pixels × (N−1) layer blends × 2 `mul` + `div` ops each = slow
- No CPU cache-line prefetch or vectorisation.

## Decision

Implement `blend_layers` with a three-tier acceleration strategy:

### 1. SIMD via `wide::u32x4`

Process 4 pixels simultaneously by splitting each `Color32` into four
lane-parallel `u32x4` vectors (R, G, B, A). The blend formula becomes:

```rust
// Per channel, 4 pixels at once:
let inv_a = u32x4::splat(255) - top_a;
acc_r = top_r + (((acc_r * inv_a + 128) * 257) >> 16);
```

This uses fixed-point arithmetic `(value * alpha + 128) * 257 >> 16` to
divide by 255 — achieving the same precision as `f32` division without
floating-point overhead, all in SIMD.

### 2. Rayon parallel chunks

Split the output buffer into 64-byte (16-pixel) chunks processed by
rayon's work-stealing thread pool:

```rust
buf_aligned.par_chunks_mut(16).enumerate().for_each(|(chunk_idx, out)| { ... });
```

A minimum threshold (`PARALLEL_BLEND_THRESHOLD = 64` chunks) prevents
parallelism overhead from dominating small dirty-rect blends.

### 3. Scalar head/tail fallback

Pixels before the first 4-aligned boundary and after the last full chunk
are processed with scalar `alpha_blend`. This avoids out-of-bounds SIMD
reads on non-aligned canvases.

The `wide` crate was chosen over `std::simd` (nightly-only, unstable) and
`core_simd` (Rust nightly feature gate) because it is stable, requires no
nightly features, and wraps compiler intrinsics (SSE2/AVX2 on x86, NEON
on ARM) directly.

## Consequences

- **Positive:** ~4× throughput on a single core (SIMD) × number of cores
  (rayon) = ~16–32× speedup over scalar blend on a 4-core machine.
- **Positive:** Single-layer fast-path (`layers.len() == 1`) uses
  `bytemuck::cast_slice` for a direct `memcpy`, completely bypassing SIMD.
- **Positive:** The `blend_region` function reuses the same SIMD machinery
  for dirty-rect partial blends, avoiding a separate code path.
- **Negative:** 4-pixel alignment constraint adds complexity in
  `blend_pixel_range` (head/tail scalar loops).
- **Negative:** `wide::u32x4` is a third-party dependency with its own
  maintenance risk; migrating to stable `std::simd` in the future would
  require rewriting the inner loop.
- **Negative:** Rayon's work-stealing adds ~1 μs of scheduling latency per
  parallel call — negligible for full-canvas blends but measurable for
  tiny dirty-rects (mitigated by the threshold).
