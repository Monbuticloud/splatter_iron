# brush_library

Persistent collection of custom brush tips with naming, thumbnails, and
on-disk storage via PNG files + a JSON index file.

Analogous to `StampLibrary`, but for custom brush tips imported from
`.abr`, `.gbr`, or created programmatically.

## Data directory

The library is rooted at `{data_dir}/brushes/`.  Two types of files live
there:

- **`index.json`** — a JSON array of entry metadata (name, filename,
  dimensions, spacing, selection state).
- **`{nanos}.png`** — individual brush tip images, saved as sRGBA PNG
  with unmultiplied alpha.  Filenames are nanosecond timestamps for
  uniqueness.

## `BrushEntry`

```rust
pub struct BrushEntry {
    pub name: String,
    pub filename: String,
    pub pixels: Vec<Color32>,
    pub width: u32,
    pub height: u32,
    pub spacing: u8,
    pub texture_handle: Option<TextureHandle>,
}
```

Each entry stores the premultiplied pixel data in memory, the on-disk
PNG filename, and an optional egui texture handle for gallery preview.

## `BrushLibrary`

```rust
pub struct BrushLibrary {
    brushes: Vec<BrushEntry>,
    selected_index: Option<usize>,
    brushes_dir: PathBuf,
}
```

### Public methods

| Method | Purpose |
|---|---|
| `load_from_disk(data_dir)` | Load / create the library directory and populate entries from disk |
| `create_textures(ctx)` | Create egui textures for entries that lack one |
| `add(name, pixels, w, h, spacing, ctx)` | Persist a new brush + update index |
| `remove(index)` | Delete a brush and its PNG file |
| `select(index)` | Select a brush by index |
| `selected_index()` → `Option<usize>` | Index of the current selection |
| `selected()` → `Option<&BrushEntry>` | The selected brush entry |
| `entries()` → `&[BrushEntry]` | All entries |
| `len()` → `usize` | Entry count |
| `is_empty()` → `bool` | True if no entries |
| `get(index)` → `Option<&BrushEntry>` | Entry by index |

### Persistence flow

1. **Load**: `load_from_disk` reads `index.json`, then for each entry
   loads the corresponding PNG via the `image` crate.  Raw RGBA bytes
   are converted to premultiplied `Color32`.
2. **Add**: `add` saves the entry as a PNG file (via
   `image::save_buffer`), creates an egui texture, and appends to the
   index.
3. **Remove**: `remove` deletes the PNG file from disk and updates the
   index.  The in-memory entry is dropped, freeing its texture handle.
4. **Index write**: `save_index` serialises the current entry list to
   `index.json` as pretty-printed JSON.

### Texture caching

On each frame, `create_textures` is called to generate egui textures for
any entries that do not yet have one.  The raw premultiplied pixels are
un-premultiplied before being passed to `egui::ColorImage` because egui
expects straight alpha for image data.
