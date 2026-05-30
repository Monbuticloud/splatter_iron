# tools

Brush and fill tool implementations. Each submodule implements one drawing
primitive and exposes public functions that operate on a `&mut Canvas` and
return an `UndoRecord` for undo support.

## Submodules

| Module           | Exports                                    | Purpose                                                     |
| ---------------- | ------------------------------------------ | ----------------------------------------------------------- |
| `bucket_fill`    | `draw_bucket_fill`                         | Scanline flood-fill from a seed point                       |
| `circle_brush`   | `draw_circle`, `draw_circle_line`          | Midpoint-circle span fill (single stamp + Bresenham stroke) |
| `square_brush`   | `draw_square`, `draw_square_line`          | Rectangular fill (single stamp + Bresenham stroke)          |
| `custom_brush`   | `draw_custom_brush_line`                   | Custom brush tip line drawing with spacing                  |
| `stamp_brush`    | `draw_stamp_line`                          | External-image stamp brush                                  |
| `brush_common`   | `apply_visited_runs`                       | Shared visited-pixel run capture for all brush tools        |
| `brush_parsers`  | `parse_brush_file`                         | GBR/ABR file format parsers                                 |
### Common contract

Every public drawing function:

- Accepts a `layer` index and panics if it is out of range for `canvas.pixels`.
- Accepts an `alpha_overlay` flag — when true, blends the new colour over
  existing pixels via premultiplied-alpha compositing; when false, overwrites.
- Captures before-pixel data for all modified pixels and returns it in the
  `UndoRecord` so that strokes can be undone.
- Updates `canvas.dirty_rect` by unioning the bounding box of affected pixels.

## `brush_parsers`

Parsers for .gbr (GIMP brush) and .abr (Photoshop brush) file formats. Exports parse_brush_file public API and parse_gbr/parse_abr internal helpers.

## `custom_brush`

Custom brush line drawing from loaded brush tips. Exports `draw_custom_brush_line` which interpolates tip placements along a linear-interpolated drag path with spacing derived from the brush's `spacing_pct`.

## `stamp_brush`

External-image stamp brush tool. Exports draw_stamp_line which stamps a loaded image at interpolated positions along a drag line.
