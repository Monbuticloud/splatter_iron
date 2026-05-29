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
