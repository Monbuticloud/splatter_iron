# ADR 23: Brush Parser Subsystem — Current State

- **Status:** Accepted
- **Date:** 2026-06-01

## Context

ADR 17 designed a brush parser subsystem with several specific predictions
about the final implementation:

| Predicted in ADR 17 | Actual implementation |
|---|---|
| `ParsedBrush` struct name | `BrushTip` |
| `ParseError` error type | `String` error type |
| Magic-byte format detection | Extension-based dispatch |
| Directory module (`brush_parsers/`) | Flat file (`brush_parsers.rs`) |
| `parse_abr`/`parse_gbr` as private `fn` | Both are `pub(crate)` |

The actual implementation diverged from the design document in several
significant ways.

## Decision

The brush parser subsystem lives in `src/tools/brush_parsers.rs` as a single
flat module, not a directory. The public API is:

### BrushTip

```rust
pub struct BrushTip {
    pub name: String,
    pub pixels: Vec<Color32>,
    pub width: u32,
    pub height: u32,
    pub spacing: u8,
}
```

- `String` error type throughout (no dedicated `ParseError`).
- `spacing` defaults to 25 when unknown.

### Extension-based dispatch

```rust
pub fn parse_brush_file(path: &Path) -> Result<Vec<BrushTip>, String> {
    let ext = path.extension()...;
    match ext.as_str() {
        "gbr" => parse_gbr(&data)?,
        "abr" => parse_abr(&data)?,
        _ => return Err(format!("Unsupported brush format: .{ext}")),
    }
}
```

Unlike the magic-byte dispatch predicted in ADR 17, the current code trusts
the file extension to select the parser. Both `parse_gbr` and `parse_abr`
still validate magic bytes internally as a secondary check.

### Internal parsers

Both `parse_gbr` and `parse_abr` are `pub(crate)` (not `pub` and not private),
allowing direct testing and use within the crate.

- **parse_gbr**: Supports GBR versions 1 (20-byte header) and 2 (24-byte
  header + spacing field). Handles 1 bpp (grayscale → white+alpha) and
  4 bpp (RGBA straight-alpha) pixel data. Validates GBR magic bytes
  internally.
- **parse_abr**: Supports ABR versions 6–10. Scans subblocks by 8BIM
  marker, extracting sampled brushes (embedded PNG or raw BGRA) and
  rasterising computed brushes (round, capped-round, square, diamond
  shapes with hardness/roundness/angle/diameter parameters).
- **Computed brush rasterisation**: Shape rendering functions
  (`rasterise_round`, `rasterise_square`, `rasterise_diamond`,
  `rasterise_capped_round`) produce anti-aliased, hardness-controlled
  alpha masks — a significant feature not anticipated in ADR 17.

### No dedicated ParseError

Errors use `Result<_, String>` throughout. Callers (brush library,
file-IO pipeline) format the error string for display in the UI's error
overlay. This trades type safety for simplicity — there is only one
failure mode (parse failure) and the consumer only needs a message.

## Consequences

- **Positive:** The flat-file structure is simpler to navigate than a
  directory module — all parsers and rasterisers are visible in one file.
- **Positive:** Extension-based dispatch is faster (no file open + magic
  read) and matches user expectations.
- **Positive:** Computed brush rasterisation handles the most common ABR
  brush shapes (round, capped-round, square, diamond) that real-world
  .abr files use — the pure-pixel-sampling approach predicted in ADR 17
  would have rejected these entirely.
- **Negative:** Extension-based dispatch fails on renamed files (e.g.
  `.abr.png`), unlike the magic-byte approach predicted in ADR 17.
- **Negative:** The flat file is 593 lines and growing — a directory module
  with separate files for `gbr.rs`, `abr.rs`, `raster.rs` would improve
  maintainability.
- **Negative:** `String` error type loses structured error information
  (no way to match on error kind at the call site).
