# brush_parsers

Brush file format parsers for GIMP Brush (`.gbr`) and Photoshop Brush
(`.abr`). The public entry point is `parse_brush_file`, which dispatches
to the format-specific parser based on file extension.

## `fn parse_brush_file`

```rust
pub fn parse_brush_file(path: &Path) -> Result<Vec<BrushTip>, String>
```

Opens the file at `path`, inspects its extension, and calls the
appropriate internal parser. Returns one or more `BrushTip` values.

### Supported extensions

| Extension | Parser      | Details                                                                                           |
| --------- | ----------- | ------------------------------------------------------------------------------------------------- |
| `.gbr`    | `parse_gbr` | GIMP Brush v1 (20-byte header) and v2 (24-byte header), 1 bpp grayscale or 4 bpp RGBA             |
| `.abr`    | `parse_abr` | Photoshop Brush v6–10, sampled brushes (embedded PNG or raw BGRA) and computed parametric brushes |

## GBR Format

```
Offset  Size  Field
 0       4     Magic ("GIMP")
 4       4     Version (u32 BE, 1 or 2)
 8       4     Width (u32 BE)
12       4     Height (u32 BE)
16       4     Bytes per pixel (u32 BE, 1 or 4)
20       4     Spacing (u32 BE, v2 only)
24+            Pixel data (row-major)
```

Grayscale (1 bpp) pixels are treated as white with the grayscale value
as alpha. RGBA (4 bpp) pixels are straight-alpha and are converted to
premultiplied alpha internally.

## ABR Format

### Header

```
Offset  Size  Field
 0       4     Magic ("8BPB")
 4       2     Version (u16 BE, 6–10)
 6       2     Sub‑block count (u16 BE)
 8       6     Reserved / format variant
```

### Sub‑blocks

Each sub‑block starts with a 14‑byte header:

```
Offset  Size  Field
 0       4     Signature ("8BIM")
 4       2     Block type (u16 BE: 1 = sampled, 2 = computed)
 6       4     Block size (u32 BE)
10       4     Reserved
```

Two block types are recognised:

- **Sampled brush** (`block_type = 1`): the block data contains tag-based
  metadata. A tag with an `8BIM` extended signature or a simple tag may
  hold embedded image data, which is decoded first as PNG (preferred) and
  then as raw BGRA.

- **Computed brush** (`block_type = 2`): the block data contains parametric
  tags (`diam`, `hrad`, `rond`, `angl`, `spac`, `shpe`) that describe a
  brush to be rasterised procedurally. Each tag is 4‑byte name + 4‑byte
  length (u32 BE) + value.

### Sampled brush decoding

Embedded images are decoded via:

1. **PNG** — recognised by the `0x89PNG...` header, decoded via the `image`
   crate.
2. **Raw BGRA** — pixel data interpreted as BGRA bytes, converted to
   premultiplied RGBA. Dimensions are estimated by rounding the square
   root of the pixel count.

### Computed brush rasterisation

| Shape        | Tag value | Rasteriser               | Description                                         |
| ------------ | --------- | ------------------------ | --------------------------------------------------- |
| Round        | `shpe=0`  | `rasterise_round`        | Filled circle with hardness-controlled edge falloff |
| Square       | `shpe=1`  | `rasterise_square`       | Filled square with hardness-controlled edge falloff |
| Diamond      | `shpe=2`  | `rasterise_diamond`      | 45° rotated filled square                           |
| Capped round | `shpe=3`  | `rasterise_capped_round` | Rectangle with semicircular end caps                |

All rasterisers accept `hardness` (0–100%, mapped from the `hrad` tag)
which controls the distance from the brush centre that remains fully
opaque before linear falloff to transparent at the edge.

### Default spacing

If a computed brush does not include a `spac` tag, spacing defaults
to 25 (25 %). Raw BGRA sampled brushes also default to 25.
PNG‑embedded sampled brushes use the spacing from their native format
or default to 25.

## `BrushTip`

```rust
pub struct BrushTip {
    pub name: String,
    pub pixels: Vec<Color32>,
    pub width: u32,
    pub height: u32,
    pub spacing: u8,
}
```

Represents a single decoded brush tip ready for use in the brush library.
`pixels` are stored in premultiplied-alpha format.
