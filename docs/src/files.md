# files

## `load_data_from_file(path)`

Reads the raw bytes of a file from disk. A thin wrapper around
`std::fs::read(path)` that serves as the I/O entry point for both loading
canvas files and importing images.

### Signature

```rust
pub fn load_data_from_file(path: &Path) -> Result<Vec<u8>, std::io::Error>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `path` | `&Path` | Filesystem path to read |

### Errors

Returns `std::io::Error` if:
- The file does not exist at `path`.
- The calling process lacks read permission.
- The path points to a directory or a non-regular file.
- An I/O error occurs during the read.

---

## `load_app_from_data(data)`

Deserialises a `Canvas` from zstd-compressed JSON bytes. This is the
deserialisation counterpart to `save_canvas_to_bytes` and handles both
decompression and JSON parsing.

### Signature

```rust
pub fn load_app_from_data(data: &[u8]) -> anyhow::Result<Canvas>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `data` | `&[u8]` | Zstd-compressed JSON bytes produced by `save_canvas_to_bytes` |

### Errors

Returns an error if:
- The input is not valid zstd-compressed data.
- The decompressed bytes are not valid UTF-8 JSON.
- The JSON structure does not match the `Canvas` type (e.g. missing fields,
  wrong field types).

---

## `save_canvas_to_bytes(canvas)`

Serialises a `Canvas` to zstd-compressed JSON bytes without writing to disk.
Uses multi-threaded (zstdmt) compression at level 10. This is intentionally
split from `save_bytes_to_file` so that the CPU-heavy compression can be
offloaded to a background thread.

### Signature

```rust
pub fn save_canvas_to_bytes(canvas: &Canvas) -> anyhow::Result<Vec<u8>>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `canvas` | `&Canvas` | The canvas to serialise |

### Errors

Returns an error if:
- JSON serialisation of the `Canvas` fails (should not happen in practice since
  `Canvas` derives `Serialize` and contains only plain data).
- Zstd compression fails (e.g. out of memory).

### Performance notes

- Compression level `10` provides a good trade-off between file size and speed
  for `.splattercanvas` files.
- The thread count is set to `available_parallelism()`, falling back to 1 if
  the system cannot report the number of hardware threads.
- The encoder uses zstdmt for multi-threaded compression.

---

## `save_bytes_to_file(data, path)`

Writes pre-serialised bytes to a file. A pure I/O operation with no
computation. Designed to be called on the main thread (or any thread) after
`save_canvas_to_bytes` has completed compression on a background thread.

### Signature

```rust
pub fn save_bytes_to_file(data: &[u8], path: &Path) -> anyhow::Result<()>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `data` | `&[u8]` | Pre-serialised bytes (e.g. from `save_canvas_to_bytes`) |
| `path` | `&Path` | Destination file path |

### Errors

Returns an error if:
- The parent directory does not exist.
- The calling process lacks write permission.
- An I/O error occurs during the write (disk full, etc.).

---

## `export_as_image(premultiplied_rgba, width, height, path, format)`

Exports a flattened premultiplied RGBA pixel buffer to a file in one of 13
supported image formats.

For JPEG the alpha channel is blended against a white background so the image
is fully opaque. For all other formats the premultiplied pixels are converted to
straight (unmultiplied) alpha.

### Signature

```rust
pub fn export_as_image(
    premultiplied_rgba: &[u8],
    width: u32,
    height: u32,
    path: &Path,
    format: image::ImageFormat,
) -> anyhow::Result<()>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `premultiplied_rgba` | `&[u8]` | Flattened premultiplied RGBA bytes (e.g. from `output_rgba`) |
| `width` | `u32` | Image width in pixels |
| `height` | `u32` | Image height in pixels |
| `path` | `&Path` | Destination file path (extension determines format only for the file system; the `format` parameter is authoritative) |
| `format` | `image::ImageFormat` | Target image format |

### Supported formats

| Format | Variant | Alpha handling | Notes |
|---|---|---|---|
| AVIF | `AvifEncoder` | Straight alpha | Lossy by default |
| PNG | `PngEncoder` | Straight alpha | Lossless |
| JPEG | `JpegEncoder` | Blended against white | Quality 100 |
| WebP | `WebPEncoder::new_lossless` | Straight alpha | Lossless variant |
| GIF | `GifEncoder` | Straight alpha | Single frame |
| TIFF | `TiffEncoder` | Straight alpha | — |
| TGA | `TgaEncoder` | Straight alpha | — |
| ICO | `IcoEncoder` | Straight alpha | — |
| PNM | `PnmEncoder` | Straight alpha | — |
| QOI | `QoiEncoder` | Straight alpha | Quite OK Image |
| OpenEXR | `OpenExrEncoder` | Straight alpha | HDR format |
| HDR | `HdrEncoder` | RGB float (alpha ignored) | RGB32F, linear |
| Farbfeld | `FarbfeldEncoder` | Straight alpha | RGBA16, native endian |

### JPEG special case

For JPEG, the premultiplied alpha is un-done by blending against white:
`r' = r + (255 - a)`, clamped. The resulting RGB8 image has no alpha channel.

### HDR special case

HDR stores linear float RGB. Each u8 channel is divided by 255.0 to recover a
normalised float, then written as an RGB32F pixel. Alpha is discarded.

### Farbfeld special case

Farbfeld requires u16 per channel, native endian. Each u8 value is widened to
u16 and written as RGBA16.

### Errors

Returns an error if:
- The file cannot be created at `path`.
- The chosen image encoder fails.
- `format` is not one of the 13 supported variants (the `_` arm bails with
  `"Unsupported export format"`).

---

## `import_image_as_canvas(path)`

Decodes an image file into a single-layer `Canvas`. Supports any raster format
that the `image` crate can decode (PNG, JPEG, WebP, GIF, BMP, TIFF, etc.).

The resulting canvas has one layer containing premultiplied-alpha RGBA pixels at
the image's native resolution. `render_next_frame` is set to `true` so the
compositor produces the initial blend.

### Signature

```rust
pub fn import_image_as_canvas(path: &Path) -> anyhow::Result<Canvas>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `path` | `&Path` | Path to the image file to import |

### Pipeline

1. `image::open(path)` decodes the file into a `DynamicImage`.
2. `to_rgba8()` converts to a flat RGBA buffer (even for grayscale/CMYK
   sources).
3. Each pixel is converted from straight RGBA to premultiplied-alpha via
   `premultiply()`.
4. A single-layer `Canvas` is constructed with the premultiplied pixel data.

### Errors

Returns an error if:
- The file cannot be read.
- The image format is not recognised by the `image` crate.
- The file is corrupted or truncated.
