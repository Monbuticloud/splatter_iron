# undo

## `enum BeforePixels`

`BeforePixels` provides compressed storage for a run of pixels as they existed before a stroke was applied. Rather than always storing a full `Vec<Color32>`, runs whose pixels are all identical are stored as a single `Color32` value, saving memory and improving cache performance for the common case of drawing over a uniform background.

### Variants

| Variant | Data | When used |
|---------|------|-----------|
| `All(Color32)` | Single color value | Every pixel in the run had the same original color (run is long enough to benefit from compression) |
| `Many(Vec<Color32>)` | Full pixel vector | Pixels in the run had distinct colors, or the run was too short for RLE to be worthwhile |

### Memory trade-off

`All` stores 4 bytes (one `Color32`). `Many` stores `N * 4` bytes for the vector allocation plus heap overhead. The `compress_run` function uses a threshold of 8 pixels: runs shorter than 8 are never compressed because the vector overhead dominates for very short spans. For a 100-pixel uniform run, `All` saves 396 bytes versus storing the full vector.
