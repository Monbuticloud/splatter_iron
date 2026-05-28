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

## `fn unpremultiply()`

```rust
pub fn unpremultiply(color: Color32) -> Color32
```

Converts a premultiplied-alpha `Color32` back to straight alpha. This is the
inverse of [`premultiply`].

Fully opaque (`alpha == 255`) and fully transparent (`alpha == 0`) pixels
pass through unchanged — the alpha == 0 case is handled explicitly to avoid
a division-by-zero hazard.

**Algorithm:** For each colour channel `c`, the straight value is
`(c * 255) / alpha`, clamped to `[0, 255]`. The result is re-packed via
`Color32::from_rgba_premultiplied` (which accepts the RGBA values as-is;
the "premultiplied" in the constructor name is a misnomer for this use
case — the alpha channel is preserved unchanged).

**Use in the pipeline:** This function is called when the user selects a
colour from the UI colour picker (which returns straight-alpha) and the
code needs to convert existing premultiplied swatches for display, or when
exporting to image formats that expect straight-alpha data.

## `fn premultiply()`

```rust
pub const fn premultiply(color: Color32) -> Color32
```

Converts a straight-alpha `Color32` to premultiplied alpha. This is the
inverse of [`unpremultiply`].

Premultiplied alpha is the internal representation used throughout
SplatterIron's blending pipeline — the `alpha_blend` function, SIMD blend
chunks, and the layer compositor all assume premultiplied inputs. Using
premultiplied alpha avoids colour-bleeding artifacts at semi-transparent
edges and simplifies the over compositing operation to a single multiply-add.

**Algorithm:** Uses fixed-point arithmetic `(c * alpha + 128) * 257 >> 16`
for each colour channel, which implements correct rounding division by 255
without a hardware division instruction. Fully opaque (`alpha == 255`)
passes through unchanged. Fully transparent returns `Color32::TRANSPARENT`.

**Caller invariant:** The input must be straight (non-premultiplied) RGBA.
Calling this on an already-premultiplied pixel will darken colours further.

**Use in the pipeline:** Called when the user picks a colour or when brush
tools produce new pixel data that must be composited into the layer stack.
