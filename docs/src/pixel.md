# pixel

Premultiplied-alpha pixel blending with SIMD (`wide::u32x4`) and rayon
parallelism. Provides `blend_layers` (full-canvas) and `blend_region`
(dirty-rect) compositing, plus `premultiply`, `unpremultiply`, and
`alpha_blend` primitives.

The module is the compositing backend for the layer stack in `document.rs`.
Every frame, the UI asks `Document` for a rendered RGBA texture; `Document`
calls either `blend_layers` or `blend_region` to produce it.

## `BYTES_PER_PIXEL`

```rust
pub const BYTES_PER_PIXEL: usize = 4;
```

Each pixel in the output RGBA byte buffer occupies 4 bytes (one byte each for
red, green, blue, and alpha). This constant is used throughout the blending
pipeline to compute byte-level offsets from pixel indices, most notably in
`blend_pixel_range` where output slices are indexed as
`output[pixel_start * BYTES_PER_PIXEL..]`.

The value 4 corresponds to the `Color32` representation in egui, which stores
packed RGBA as `[u8; 4]`.

## `F32_COLOR_MAX`

```rust
pub const F32_COLOR_MAX: f32 = 255.0;
```

The maximum `u8` colour channel value, expressed as `f32`, used when
normalising byte-level channels into floating-point computations. This
constant documents the implicit conversion factor between the `[0, 255]`
`u8` domain and the `[0.0, 1.0]` `f32` domain used in colour-space
conversions or when interfacing with APIs that accept floating-point
colour components.
