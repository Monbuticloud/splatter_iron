# ADR 17: Brush Parser Subsystem for GBR/ABR Files

- **Status:** Accepted
- **Date:** 2026-05-30

## Context

SplatterIron allows importing brush tips from industry-standard file formats:
GIMP brush (`.gbr`) and Adobe Photoshop brush (`.abr`). These formats have
different binary layouts, colour depths, compression schemes, and metadata
(brush name, spacing).

Embedding both parsers in a single file would create a large module with two
unrelated parsing state machines. Inlining parse logic at the call site (e.g.
in `brush_library`) would couple the asset persistence layer to binary format
details.

## Decision

Create a dedicated `tools::brush_parsers` submodule with one public entry
point and internal helpers for each format:

```
src/tools/brush_parsers/
    mod.rs          — public `parse_brush_file()` dispatcher
    parse_gbr()     — .gbr format parser (internal)
    parse_abr()     — .abr format parser (internal)
```

### Public API

```rust
/// Parse a brush file and return a vector of brush entries.
pub fn parse_brush_file(path: &Path) -> Result<Vec<ParsedBrush>, ParseError>
```

`ParsedBrush` is a format-agnostic output:

```rust
pub struct ParsedBrush {
    pub name: String,
    pub pixels: Vec<Color32>,
    pub width: u32,
    pub height: u32,
    pub spacing: u8,
}
```

### Key design choices

1. **Format detection by magic bytes**: `parse_brush_file` reads the first
   few bytes of the file to determine the format, then dispatches to the
   correct internal parser. No filename extension guessing.

2. **Unified output type**: Both `.gbr` and `.abr` parsers produce the same
   `ParsedBrush` struct, so callers (the brush library import flow) don't
   need to know which format produced the result.

3. **Internal parsing functions**: `parse_gbr` and `parse_abr` are module-
   private (`fn` not `pub fn`), keeping the public surface minimal. They
   are tested through the public `parse_brush_file` entry point or directly
   in the test module.

4. **Layer separation**: The brush import flow in `brush_library` calls
   `parse_brush_file`, converts `ParsedBrush` entries into `BrushEntry`,
   then passes them to `Library::add_entry`. The parser never touches the
   library; the library never reads raw brush files.

## Consequences

- **Positive:** Adding a new format (e.g. `.jbr` for Krita) means adding one
  internal parser function and one branch in `parse_brush_file` — no changes
  to brush library or asset infrastructure.
- **Positive:** Each parser can be unit-tested independently with binary
  fixtures.
- **Positive:** The `ParsedBrush` output type decouples parsing from storage
  — the same output could be used for preview rendering before saving.
- **Positive:** Magic-byte dispatch is more robust than extension-based
  dispatch; renamed files (`.abr.png`) still parse correctly.
- **Negative:** Two format parsers share no code — each must implement its own
  byte-level reading, colour conversion, and metadata extraction.
- **Negative:** The unified `ParsedBrush` discards format-specific metadata
  that might be useful later (e.g. ABR brush dynamics settings), requiring
  a schema version bump if such fields are needed.
