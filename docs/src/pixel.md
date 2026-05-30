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

## `fn alpha_blend()`

```rust
pub const fn alpha_blend(destination: Color32, source: Color32) -> Color32
```

Composites a premultiplied source pixel over a premultiplied destination
pixel using the `over` compositing operator. Both inputs and the result
are in premultiplied-alpha format.

**Algorithm:** For each channel, computes:

```
result_channel = source_channel + (dest_channel * (255 - source_alpha) + 128) >> 8
```

The `>> 8` is a fixed-point division by 256 (with `+ 128` rounding bias),
which approximates division by `255` closely enough for 8-bit colour while
avoiding a hardware division or a 257-multiply.

**Use in the pipeline:** This is the scalar fallback for edge pixels that
don't align to the 4-pixel SIMD boundary in `blend_pixel_range`. It is also
the primitive used by `blend_region`'s row-by-row processing (which runs
sequentially since dirty rects are typically small).

## `fn blend_layers()`

```rust
pub fn blend_layers(layers: &[(&[Color32], u8)], output: &mut [u8])
```

Composite multiple premultiplied layers into a single RGBA byte buffer.
Layers are blended bottom-to-top: index 0 is the bottommost layer, and the
last index is composited on top.

This is the primary compositing entry point used for full-canvas rendering
(e.g. after an undo/redo, after loading a file, or when the `RenderState`
is `ActiveWake`).

**Performance:** The function delegates to `blend_pixel_range` with
`parallel = true`. When there are 64 or more 4-pixel SIMD chunks (i.e. 256
or more pixels), the blend is parallelised via rayon across the SIMD-aligned
body. Scalar head/tail pixels (those before/after the 4-aligned boundary)
and the single-layer fast path (`memcpy`) run on the calling thread.

**Panics:**

- If `layers` is empty.
- (debug builds only) If any layer has a different pixel count from
  `layers[0]`.
- If `output.len() != layers[0].len() * 4`.

## `fn blend_region()`

```rust
pub fn blend_region(
    layers: &[(&[Color32], u8)],
    output: &mut [u8],
    canvas_width: u32,
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
)
```

Blend only the pixels within a dirty rectangle, row by row. This is an
optimisation over `blend_layers` when only a small portion of the canvas has
changed (e.g. during a brush stroke or a bucket fill).

**Performance:** Each row segment is processed via `blend_pixel_range` with
`parallel = false`. Dirty rects from brush strokes are typically small
enough that parallel overhead would dominate — sequential iteration across
rows with scalar + SIMD within each row is the preferred strategy.

**Empty layers:** Returns immediately if `layers` is empty (no-op), unlike
`blend_layers` which panics.

**Panics:**

- If any layer has fewer pixels than required by the region bounds.
- If `output` is too small for the required byte range.
