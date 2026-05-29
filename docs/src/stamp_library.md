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
| `stamps`         | `Vec<StampEntry>`     | Stored stamp entries (private)                   |
| `selected_index` | `Option<usize>`       | Index of the currently selected stamp, if any    |
| `stamps_dir`     | `std::path::PathBuf`  | Absolute path to the stamps directory on disk     |

## `StampLibrary::load_from_disk`

```rust
pub fn load_from_disk(data_dir: &Path) -> Self
```

Load or create a stamp library rooted at `data_dir/stamps/`. If the directory
does not exist it is created. Entries are loaded from `index.json`; missing
or corrupt PNG files are silently skipped.

### Parameters

| Parameter  | Type     | Purpose                                          |
| ---------- | -------- | ------------------------------------------------ |
| `data_dir` | `&Path`  | Application data directory (parent of `stamps/`) |

### Panics

Panics if the stamps directory cannot be created.

## `StampLibrary::create_textures`

```rust
pub fn create_textures(&mut self, ctx: &egui::Context)
```

Create egui textures for all stamp entries that don't have one yet. Should
be called after loading or when the egui context is available.

### Parameters

| Parameter | Type             | Purpose                              |
| --------- | ---------------- | ------------------------------------ |
| `ctx`     | `&egui::Context` | The egui context for texture creation |

## `StampLibrary::add`

```rust
pub fn add(&mut self, name: String, pixels: Vec<Color32>, width: u32, height: u32, ctx: &egui::Context)
```

Add a new stamp to the library and persist it to disk. Creates a PNG file in
the stamps directory and updates `index.json`. A cached texture handle is
created for preview rendering.

### Parameters

| Parameter | Type             | Purpose                    |
| --------- | ---------------- | -------------------------- |
| `name`    | `String`         | Display name for the stamp |
| `pixels`  | `Vec<Color32>`   | Premultiplied-alpha pixels |
| `width`   | `u32`            | Image width in pixels      |
| `height`  | `u32`            | Image height in pixels     |
| `ctx`     | `&egui::Context` | Egui context for texture   |

## `StampLibrary::remove`

```rust
pub fn remove(&mut self, index: usize)
```

Remove the stamp at `index` from the library, delete its PNG file, and
persist the updated index. If `index` matches the current selection,
selection is cleared.

### Parameters

| Parameter | Type     | Purpose                  |
| --------- | -------- | ------------------------ |
| `index`   | `usize`  | Index of the entry to remove |

## `StampLibrary::select`

```rust
pub fn select(&mut self, index: usize)
```

Select the stamp at `index`. No-op if `index` is out of range.

## `StampLibrary::selected_index`

```rust
pub fn selected_index(&self) -> Option<usize>
```

Return the index of the currently selected stamp, if any.

## `StampLibrary::selected`

```rust
pub fn selected(&self) -> Option<&StampEntry>
```

Return a reference to the currently selected stamp entry, if any.

## `StampLibrary::selected_mut`

```rust
pub fn selected_mut(&mut self) -> Option<&mut StampEntry>
```

Return a mutable reference to the currently selected stamp entry, if any.

## `StampLibrary::entries`

```rust
pub fn entries(&self) -> &[StampEntry]
```

Return a slice of all stamp entries.

## `StampLibrary::len`

```rust
pub fn len(&self) -> usize
```

Return the number of stamps in the library.

## `StampLibrary::is_empty`

```rust
pub fn is_empty(&self) -> bool
```

Return `true` if the library is empty.

## `StampLibrary::get`

```rust
pub fn get(&self, index: usize) -> Option<&StampEntry>
```

Return a reference to the stamp at `index`, or `None`. |
