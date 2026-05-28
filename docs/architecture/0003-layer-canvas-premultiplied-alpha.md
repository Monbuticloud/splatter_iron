# ADR 3: Layer-Based Canvas with Premultiplied Alpha

- **Status:** Accepted
- **Date:** 2026-05-16
- **Commits:** `59c2fde`, `05bc6ed`

## Context

A paint program's core data structure is its canvas: a grid of pixels organized
into layers that can be reordered, hidden, or deleted independently. Two design
decisions shape everything downstream (blending, undo, serialization, GPU
upload):

1. **Layer model**: Are layers stored as `Vec<Layer>` with each layer owning
   its own `Vec<Color32>`? Or as a single flat buffer with layer metadata?
2. **Color space**: Are pixels stored as straight alpha or premultiplied alpha?
   Premultiplied alpha stores `(R*A, G*A, B*A, A)` instead of `(R, G, B, A)`,
   which eliminates dark fringing at alpha boundaries and halves the blend
   arithmetic.

## Decision

1. **Layer model**: `Canvas` owns a `Vec<Layer>` where each `Layer` is a flat
   `Vec<Color32>` in row-major order. Layers are composited bottom-to-top
   (index 0 = background). This was the model from the initial commit
   (`0de9592`) and was extended in `05bc6ed` with `add_layer`, `delete_layer`,
   `move_layer_up/down`, and `select_layer` in a new `Document` wrapper.

2. **Color space**: All pixel operations use premultiplied alpha internally.
   Brush colors are premultiplied on input (`premultiply()`), the compositing
   pipeline operates on premultiplied `Color32` values, and output to egui uses
   `egui::ColorImage::from_rgba_premultiplied`. Export unpremultiplies via
   `unpremultiply()` for formats that expect straight alpha (PNG, etc.).

```rust
// Before (straight alpha): 3 float ops + 3 divisions per blend
let out_a = sa + da * (1.0 - sa);
let r = ((src.r * sa + dst.r * da * (1.0 - sa)) / out_a) as u8;

// After (premultiplied): 3 integer multiplies + 0 divisions
let r = src.r + ((dst.r * inverse_src_a) / 255);
```

Alternatives considered:
- **Flat buffer with layer bitmask**: Would save memory for mostly-transparent
  layers but complicates reorder, undo, and serialization.
- **Straight alpha throughout**: Simpler conceptually but requires per-pixel
  division in the blend loop and produces dark fringing when downscaling or
  compositing semi-transparent edges.
- **Float HDR internal format**: More accurate for gradient-heavy painting but
  quadruples memory (f32 vs u8) and complicates GPU upload.

## Consequences

- **Positive:** Premultiplied blend is ~2× faster than the straight-alpha
  float version; replaces `f32` divides with integer multiply-shift.
- **Positive:** No dark fringing at alpha boundaries — crucial for anti-aliased
  brush strokes on transparent layers.
- **Positive:** `Vec<Layer>` model maps directly to `Vec<&[Color32]>` in
  `blend_layers`, enabling the SIMD + rayon path (ADR-0004).
- **Negative:** All external color input (color picker, image import, export)
  must convert via `premultiply()`/`unpremultiply()`, adding a conversion pass.
- **Negative:** File serialization stores premultiplied values; if another
  program reads the `.splattercanvas` format, it must know to unpremultiply.
- **Negative:** Each layer allocates `width * height * 4` bytes even if the
  layer is mostly transparent. For a 2000×1500 canvas, that's ~11.4 MB per
  layer.
