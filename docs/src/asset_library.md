# asset_library

Generic persistent asset library with on-disk storage via PNG files + JSON
index. Both [`BrushLibrary`] and [`StampLibrary`] are thin type aliases
around `Library<BrushEntry>` / `Library<StampEntry>` respectively.

## `trait AssetEntry`

Behaviour that an asset entry must implement for storage in a [`Library`].

| Method                 | Returns                      | Purpose                                    |
| ---------------------- | ---------------------------- | ------------------------------------------ |
| `name()`               | `&str`                       | Display name                               |
| `name_mut()`           | `&mut String`                | Mutable display name reference             |
| `filename()`           | `&str`                       | On-disk PNG filename (relative to lib dir) |
| `filename_mut()`       | `&mut String`                | Mutable filename reference                 |
| `pixels()`             | `&[Color32]`                 | Premultiplied-alpha pixel data (row-major) |
| `pixels_mut()`         | `&mut Vec<Color32>`          | Mutable pixel buffer reference             |
| `width()`              | `u32`                        | Image width in pixels                      |
| `height()`             | `u32`                        | Image height in pixels                     |
| `texture_handle()`     | `&Option<TextureHandle>`     | Cached egui preview texture                |
| `texture_handle_mut()` | `&mut Option<TextureHandle>` | Mutable texture handle reference           |
| `dir_name()`           | `&'static str`               | Subdirectory name (e.g. `"brushes"`)       |
| `json_field_name()`    | `&'static str`               | JSON index field name (e.g. `"brushes"`)   |
| `extra_index_fields()` | `Vec<(&'static str, Value)>` | Extra key-value pairs for index-file entry |
| `from_parts()`         | `Self`                       | Reconstruct from index data + decoded PNG  |

## `struct Library<T: AssetEntry>`

Persistent collection of assets with on-disk storage.

| Field            | Type            | Purpose                                       |
| ---------------- | --------------- | --------------------------------------------- |
| `entries`        | `Vec<T>`        | Asset entries (private)                       |
| `selected_index` | `Option<usize>` | Index of the currently selected entry, if any |
| `dir`            | `PathBuf`       | Absolute path to the library data directory   |

### Public methods

| Method                               | Purpose                                                    |
| ------------------------------------ | ---------------------------------------------------------- |
| `load_from_disk(data_dir)`           | Load / create library and populate entries from index.json |
| `create_textures(ctx)`               | Create egui textures for entries that lack one             |
| `add_entry(entry, ctx)`              | Persist a new entry + PNG + update index                   |
| `remove(index)`                      | Delete an entry and its PNG file                           |
| `select(index)`                      | Select an entry by index                                   |
| `selected_index()` → `Option<usize>` | Index of the current selection                             |
| `selected()` → `Option<&T>`          | The selected entry                                         |
| `selected_mut()` → `Option<&mut T>`  | Mutable reference to the selected entry                    |
| `entries()` → `&[T]`                 | All entries                                                |
| `len()` → `usize`                    | Entry count                                                |
| `is_empty()` → `bool`                | True if no entries                                         |
| `get(index)` → `Option<&T>`          | Entry by index                                             |

### Persistence flow

1. **Load**: `load_from_disk` reads `index.json` from `{data_dir}/{T::dir_name()}/`,
   then for each entry loads the corresponding PNG via the `image` crate.
   Raw RGBA bytes are converted to premultiplied `Color32`. Missing or
   corrupt PNG files are silently skipped.
2. **Add**: `add_entry` saves the entry as a PNG file (via `image::save_buffer`),
   creates an egui texture, and appends to the index.
3. **Remove**: `remove` deletes the PNG file from disk and updates the index.
4. **Index write**: `save_index` serialises the current entry list to
   `index.json` as pretty-printed JSON.

### Texture caching

On each frame, `create_textures` is called to generate egui textures for
any entries that do not yet have one. The raw premultiplied pixels are
un-premultiplied before being passed to `egui::ColorImage` because egui
expects straight alpha for image data.
