# ADR 1: MiMalloc Global Allocator

- **Status:** Accepted
- **Date:** 2026-05-28
- **Commit:** `7189c51`

## Context

SplatterIron performs intensive heap allocation during pixel blending (layer
compositing, undo-record capture) and image I/O (save/load/export). The default
system allocator is not specialized for the allocation patterns found in a
real-time paint program: many short-lived medium-sized buffers (canvas layers,
output RGBA, serialized zstd frames) and occasional large allocations (image
imports, full-canvas saves).

An earlier `TrackingAllocator` wrapper (commit `9fc4b1d`) wrapped MiMalloc with
`AtomicUsize` counters to track live, total, and peak allocation. This provided
debugging insight but added `unsafe` code (three `unsafe impl GlobalAlloc`
methods) and a memory-ordering fence on every allocation and deallocation —
unnecessary overhead in a release build.

## Decision

Replace `TrackingAllocator` with a bare global allocator:

```rust
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
```

Eliminating:
- 3 `unsafe` function implementations (`alloc`, `dealloc`, `realloc`)
- 3 `AtomicUsize` counters with `Ordering::Relaxed` operations per
  allocation/deallocation
- The `allocated_bytes()` public function and its `println!` diagnostics on
  exit

MiMalloc was chosen over alternatives (jemalloc, system malloc) because:
- **Fast small allocations** — free-list sharding and bump-pointer allocation
  for small sizes matches the many ~16-byte `Color32` pixel writes during brush
  strokes.
- **Fast deallocation** — batch-free lists reduce contention during layer
  deletion and undo-stack eviction.
- **Cross-platform** — first-class support on Windows, macOS, and Linux via the
  `mimalloc` crate.
- **Zero-config** — no environment variables or configuration files needed.

## Consequences

- **Positive:** Zero `unsafe` code in the allocation path; simpler `main.rs`.
- **Positive:** Slightly faster hot-path allocation due to removed atomic
  counters.
- **Negative:** No runtime visibility into heap usage (lost diagnostic output).
- **Trade-off:** The performance gain is marginal for most operations but
  matters during tight loops in `blend_pixel_range` where thousands of
  intermediate allocations per frame are avoided by MiMalloc's fast paths.
