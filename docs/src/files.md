# files

## `write_canvas(canvas, writer)`

Serialises a `Canvas` into any `std::io::Write` sink by streaming JSON directly
into a multi-threaded zstd encoder. This is the core streaming writer used by
both `save_canvas_to_bytes` and `save_canvas_to_path`.

### Signature

```rust
fn write_canvas(canvas: &Canvas, writer: impl Write) -> anyhow::Result<()>
```

### Parameters

| Parameter | Type         | Description                                  |
| --------- | ------------ | -------------------------------------------- |
| `canvas`  | `&Canvas`    | The canvas to serialise                      |
| `writer`  | `impl Write` | Destination writer (`File`, `Vec<u8>`, etc.) |

### Errors

Returns an error if JSON serialisation or zstd compression fails.

---

## `read_canvas(reader)`

Decompresses and deserialises a `Canvas` from any `std::io::Read` source by
streaming the zstd decoder directly into `serde_json::from_reader`. This is the
core streaming reader used by both `load_canvas_from_bytes` and `load_canvas_from_path`.

### Signature

```rust
fn read_canvas(reader: impl Read) -> anyhow::Result<Canvas>
```

### Parameters

| Parameter | Type        | Description                                            |
| --------- | ----------- | ------------------------------------------------------ |
| `reader`  | `impl Read` | Source of zstd-compressed JSON bytes (`File`, `&[u8]`) |

### Errors

Returns an error if:

- The input is not valid zstd-compressed data.
- The decompressed data exceeds `MAX_DECOMPRESSED_BYTES` (512 MiB).
- The JSON structure does not match the `Canvas` type (e.g. missing fields,
  wrong field types).
- The canvas has invalid dimensions (zero width/height) or a layer with the
  wrong pixel count.

---

## `save_canvas_to_bytes(canvas)`

Serialises a `Canvas` to zstd-compressed JSON bytes without writing to disk.

### Signature

```rust
pub fn save_canvas_to_bytes(canvas: &Canvas) -> anyhow::Result<Vec<u8>>
```

### Parameters

| Parameter | Type      | Description             |
| --------- | --------- | ----------------------- |
| `canvas`  | `&Canvas` | The canvas to serialise |

### Performance notes

Internally streams JSON directly into the zstd encoder via `write_canvas` — no
intermediate `Vec<u8>` for the raw JSON is materialised. Only the final
compressed buffer is allocated.

---

## `save_canvas_to_path(canvas, path)`

Serialises a `Canvas` directly to a file by streaming JSON through zstd
compression into a `File` — zero intermediate `Vec<u8>` allocations.

This is the preferred save API. The file is created at `path`; the parent
directory must exist.

### Signature

```rust
pub fn save_canvas_to_path(canvas: &Canvas, path: &Path) -> anyhow::Result<()>
```

### Parameters

| Parameter | Type      | Description             |
| --------- | --------- | ----------------------- |
| `canvas`  | `&Canvas` | The canvas to serialise |
| `path`    | `&Path`   | Destination file path   |

### Errors

Returns an error if:

- The file cannot be created (e.g. permission denied, parent directory missing).
- JSON serialisation or zstd compression fails.

---

## `load_canvas_from_bytes(data)`

Deserialises a `Canvas` from zstd-compressed JSON bytes.

### Signature

```rust
pub fn load_canvas_from_bytes(data: &[u8]) -> anyhow::Result<Canvas>
```

### Parameters

| Parameter | Type    | Description                |
| --------- | ------- | -------------------------- |
| `data`    | `&[u8]` | Zstd-compressed JSON bytes |

### Performance notes

Streams the data through zstd + serde via `read_canvas` — no intermediate
decompression `Vec<u8>` is allocated.

---

## `load_canvas_from_path(path)`

Decompresses and deserialises a `Canvas` from a file by streaming the file
through zstd decompression into `serde_json::from_reader`. No intermediate
`Vec<u8>` for either the compressed file contents or the decompressed JSON.

This is the preferred load API.

### Signature

```rust
pub fn load_canvas_from_path(path: &Path) -> anyhow::Result<Canvas>
```

### Parameters

| Parameter | Type    | Description                      |
| --------- | ------- | -------------------------------- |
| `path`    | `&Path` | Path to a `.splattercanvas` file |

### Errors

Returns an error if:

- The file cannot be read.
- The input is not valid zstd-compressed data.
- The decompressed data exceeds `MAX_DECOMPRESSED_BYTES` (512 MiB).
- The JSON structure does not match the `Canvas` type.
- The canvas has invalid dimensions (zero width/height) or a layer with the
  wrong pixel count.

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

| Parameter            | Type                 | Description                                                                                                           |
| -------------------- | -------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `premultiplied_rgba` | `&[u8]`              | Flattened premultiplied RGBA bytes (e.g. from `output_rgba`)                                                          |
| `width`              | `u32`                | Image width in pixels                                                                                                 |
| `height`             | `u32`                | Image height in pixels                                                                                                |
| `path`               | `&Path`              | Destination file path (extension determines format only for the file system; the `format` parameter is authoritative) |
| `format`             | `image::ImageFormat` | Target image format                                                                                                   |

### Supported formats

| Format   | Variant                     | Alpha handling            | Notes                 |
| -------- | --------------------------- | ------------------------- | --------------------- |
| AVIF     | `AvifEncoder`               | Straight alpha            | Lossy by default      |
| PNG      | `PngEncoder`                | Straight alpha            | Lossless              |
| JPEG     | `JpegEncoder`               | Blended against white     | Quality 100           |
| WebP     | `WebPEncoder::new_lossless` | Straight alpha            | Lossless variant      |
| GIF      | `GifEncoder`                | Straight alpha            | Single frame          |
| TIFF     | `TiffEncoder`               | Straight alpha            | —                     |
| TGA      | `TgaEncoder`                | Straight alpha            | —                     |
| ICO      | `IcoEncoder`                | Straight alpha            | —                     |
| PNM      | `PnmEncoder`                | Straight alpha            | —                     |
| QOI      | `QoiEncoder`                | Straight alpha            | Quite OK Image        |
| OpenEXR  | `OpenExrEncoder`            | Straight alpha            | HDR format            |
| HDR      | `HdrEncoder`                | RGB float (alpha ignored) | RGB32F, linear        |
| Farbfeld | `FarbfeldEncoder`           | Straight alpha            | RGBA16, native endian |

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

| Parameter | Type    | Description                      |
| --------- | ------- | -------------------------------- |
| `path`    | `&Path` | Path to the image file to import |

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

---

## XZ-compressed `.splatterarchive` format

Functions mirroring the zstd-based `.splattercanvas` API but using xz (LZMA2)
at maximum compression (preset 9). Used for the separate archive export/import
and archive autosave streams.

### `save_canvas_to_path_xz(canvas, path)`

Serialises a `Canvas` to an xz-compressed file at `path`. Uses preset 9
(max compression, single-threaded). Zero intermediate allocations — streams
JSON directly into the xz encoder.

### `load_canvas_from_path_xz(path)`

Decompresses and deserialises a `Canvas` from an xz-compressed file.

### `save_canvas_to_bytes_xz(canvas)`

Serialises a `Canvas` to xz-compressed `Vec<u8>`.

### `load_canvas_from_bytes_xz(data)`

Deserialises a `Canvas` from xz-compressed bytes.

### `write_canvas_xz(canvas, writer)`

Core streaming writer — serialises JSON into an `XzEncoder` wrapping any
`std::io::Write`.

### `read_canvas_xz(reader)`

Core streaming reader — wraps any `std::io::Read` with `XzDecoder` and
`take(512 MiB)` before `serde_json::from_reader`.

### Errors

All read functions validate canvas dimensions and layer pixel counts,
returning `anyhow::Error` on mismatch. Decompression is limited to
`MAX_DECOMPRESSED_BYTES` (512 MiB).
