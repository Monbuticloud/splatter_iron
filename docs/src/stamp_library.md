# stamp_library

Persistent collection of stamp images with naming, thumbnails, and on-disk
storage via PNG files + JSON index.

## `enum StampTintMode`

Controls whether stamp pixels are tinted by the current tool colour during
rendering.

| Variant   | Behaviour                                                              |
| --------- | ---------------------------------------------------------------------- |
| `Original` | Use the stamp's original colours as-is (no tinting).                   |
| `Tinted`   | Multiply stamp pixels by the current tool colour before compositing.   |

## `enum StampSampling`

Pixel-sampling strategy when scaling the stamp to canvas dimensions.

| Variant   | Behaviour                                                                 |
| --------- | ------------------------------------------------------------------------- |
| `Nearest`  | Nearest-neighbour (sharp edges, pixel-art friendly).                      |
| `Bilinear` | Bilinear interpolation (smooth scaling for photographs).                  |

## `struct StampEntry`

A single stamp entry in the library.

| Field             | Type                    | Purpose                                              |
| ----------------- | ----------------------- | ---------------------------------------------------- |
| `name`            | `String`                | User-given display name                              |
| `filename`        | `String`                | On-disk PNG filename (relative to `stamps/` dir)     |
| `pixels`          | `Vec<Color32>`          | Premultiplied-alpha pixel data (row-major)           |
| `width`           | `u32`                   | Stamp image width in pixels                          |
| `height`          | `u32`                   | Stamp image height in pixels                         |
| `texture_handle`  | `Option<TextureHandle>` | Cached egui texture for preview rendering            |

### `StampEntry::texture_id`

```rust
pub fn texture_id(&self) -> Option<egui::TextureId>
```

Returns the cached texture ID if a texture has been created, or `None`
otherwise. Used during rendering to display the stamp in the gallery.

## `struct StampLibrary`

Persistent collection of stamp images with on-disk storage.

| Field            | Type                  | Purpose                                          |
| ---------------- | --------------------- | ------------------------------------------------ |
| `stamps`         | `Vec<StampEntry>`     | Stored stamp entries                             |
| `selected_index` | `Option<usize>`       | Index of the currently selected stamp, if any    |
| `stamps_dir`     | `std::path::PathBuf`  | Absolute path to the stamps directory on disk     |
