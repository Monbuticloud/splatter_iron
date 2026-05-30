# ADR 16: Generic Asset Library with AssetEntry Trait

- **Status:** Accepted
- **Date:** 2026-05-30

## Context

SplatterIron needs to manage two semantically distinct asset collections:
brush tips (`BrushLibrary`) and stamp images (`StampLibrary`). Both share
nearly identical behaviour:

- Persist entries as individual PNG files on disk.
- Maintain a JSON index file (`index.json`) for entry metadata.
- Support add/remove/select/get/traverse operations.
- Cache egui `TextureHandle`s for gallery preview rendering.
- Load lazily from a subdirectory of the app data directory.

Creating two separate structs with duplicated logic (`struct BrushLibrary`
and `struct StampLibrary` with identical fields and methods) would violate
DRY and multiply maintenance burden whenever the persistence scheme changes.

## Decision

Define a generic `Library<T>` parameterised by an `AssetEntry` trait:

```rust
pub trait AssetEntry: Sized {
    fn name(&self) -> &str;
    fn name_mut(&mut self) -> &mut String;
    fn filename(&self) -> &str;
    fn filename_mut(&mut self) -> &mut String;
    fn pixels(&self) -> &[Color32];
    fn pixels_mut(&mut self) -> &mut Vec<Color32>;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn texture_handle(&self) -> &Option<TextureHandle>;
    fn texture_handle_mut(&mut self) -> &mut Option<TextureHandle>;
    fn dir_name() -> &'static str;
    fn json_field_name() -> &'static str;
    fn extra_index_fields(&self) -> Vec<(&'static str, serde_json::Value)>;
    fn from_parts(name, filename, pixels, w, h, extra) -> Self;
}

pub struct Library<T: AssetEntry> {
    entries: Vec<T>,
    selected_index: Option<usize>,
    dir: PathBuf,
}
```

`BrushLibrary` and `StampLibrary` become type aliases with thin `add_brush` /
`add_stamp` convenience constructors:

```rust
pub type BrushLibrary = Library<BrushEntry>;
pub type StampLibrary = Library<StampEntry>;
```

### Key design choices

1. **Trait over enum**: An `AssetEntry` enum with `Brush(BrushEntry)` /
   `Stamp(StampEntry)` variants would couple the generic library to every
   entry type. The trait approach lets the library be unit-tested with a
   minimal `TestEntry` and allows third-party entry types without touching
   the library code.

2. **Associated constants over instance methods**: `dir_name()` and
   `json_field_name()` are associated functions (no `&self`) so the library
   can determine the on-disk layout without an instance. This means
   `Library::load_from_disk` works on an empty directory — it knows where
   to look based on `T::dir_name()` alone.

3. **`extra_index_fields` / `from_parts` for extension**: Entry-specific
   metadata (e.g. `spacing` for brushes) round-trips through the JSON index
   via `extra_index_fields()` on write and `from_parts()` on read, without
   the generic library needing to know about any particular field.

4. **No `Serialize`/`Deserialize` on the trait**: Serialisation is handled
   entirely by the library — it maps trait methods to JSON key-value pairs
   and calls `from_parts` on read. Entries don't need serde derives.

## Consequences

- **Positive:** Brush library and stamp library share ~300 lines of generic
  logic (add, remove, select, persist, texture caching) with zero duplication.
- **Positive:** Adding a third asset type (e.g. `PatternLibrary`, `FontLibrary`)
  requires only a new entry struct + `AssetEntry` impl — no library code changes.
- **Positive:** The generic `Library` can be unit-tested independently with a
  `TestEntry` that has no I/O dependencies.
- **Positive:** On-disk format is consistent across asset types — each gets
  its own subdirectory with the same `index.json` + PNG file scheme.
- **Negative:** Trait boilerplate — each entry type must implement 13 trait
  methods (though most are trivial one-liners).
- **Negative:** Generic monomorphisation — `Library<BrushEntry>` and
  `Library<StampEntry>` produce separate compiled copies of all methods,
  slightly increasing binary size.
- **Negative:** The `extra_index_fields`/`from_parts` round-trip uses
  `serde_json::Value` for the extra fields, losing compile-time type safety
  for entry-specific metadata.
