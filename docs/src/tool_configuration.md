# tool_configuration

## `struct ToolConfiguration`

`ToolConfiguration` holds all mutable tool state for the application: the currently selected drawing tool, brush properties (color, radius, alpha-overlay toggle), sampling and tinting modes for stamps and custom brushes, grid overlay settings, and brush stabilization parameters.

The struct is a plain data container with no methods except [`Default`]. Ownership is held by `MyApp` in `app.rs`, which passes it to the UI panels for display and mutation.

### Fields

| Field                     | Type            | Purpose                                                                                                         |
| ------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------- |
| `current_tool`            | `CurrentTool`   | The currently selected drawing tool. Determines which drawing function is called on pointer events              |
| `current_color`           | `Color32`       | Color applied by brush strokes (premultiplied-alpha)                                                            |
| `radius`                  | `u32`           | Brush radius in pixels. Controls the size of brush stamps and the brush preview overlay                         |
| `alpha_overlay`           | `bool`          | Whether strokes use alpha-overlay blending instead of opaque overwrite                                          |
| `show_brush_preview`      | `bool`          | Whether to render the brush size preview indicator on the canvas                                                |
| `stamp_sampling`          | `StampSampling` | Sampling strategy when scaling stamp images to canvas size (`Nearest` or `Bilinear`)                            |
| `stamp_tint_mode`         | `StampTintMode` | Whether stamp pixels are tinted by `current_color` (`Original` or `Tinted`)                                     |
| `brush_sampling`          | `StampSampling` | Sampling strategy when scaling custom brush tips to canvas size                                                 |
| `brush_tint_mode`         | `StampTintMode` | Whether custom brush pixels are tinted by `current_color`                                                       |
| `show_grid`               | `bool`          | Whether the pixel-grid overlay is visible on the canvas                                                         |
| `grid_size`               | `u32`           | Spacing of grid lines in canvas pixels                                                                          |
| `stabilization_enabled`   | `bool`          | Whether brush stabilization (lerped virtual cursor) is active                                                   |
| `stabilization_smoothing` | `f32`           | Smoothing strength for brush stabilization (0.0–100.0). Higher values produce smoother but more delayed strokes |

### Alpha-overlay vs opaque

The `alpha_overlay` flag affects how the brush interacts with existing pixels:

- **Opaque** (`alpha_overlay: false`): The brush color completely replaces the pixel. This is the default and is suitable for most painting.
- **Alpha overlay** (`alpha_overlay: true`): The brush color is blended over the existing pixel using premultiplied-alpha compositing. This allows the brushed color to be semi-transparent, letting the underlying image show through.

The distinction matters for undo/redo: opaque strokes can be reapplied with a bulk `fill()`, while alpha-overlay strokes require per-pixel blending, which is handled in `redo_apply`.

## `impl Default for ToolConfiguration`

Provides sensible defaults for the initial application state. Rust's `#[derive(Default)]` is not used because several fields require specific values that differ from typical type defaults (e.g., `Color32` defaults to transparent black, but the initial brush color should be opaque white).

### Default values

| Field                     | Default                                                | Rationale                                                                                                |
| ------------------------- | ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------- |
| `current_tool`            | `CurrentTool::Square`                                  | Square is the simplest and most intuitive default tool for a raster paint application                    |
| `current_color`           | `Color32::from_rgba_premultiplied(255, 255, 255, 255)` | Opaque white — a neutral starting color that shows clearly against any canvas background                 |
| `radius`                  | `100`                                                  | 100 pixels gives a brush large enough to be immediately usable and visible at typical canvas zoom levels |
| `alpha_overlay`           | `false`                                                | Opaque painting is the expected default; users opt into alpha blending consciously                       |
| `show_brush_preview`      | `true`                                                 | Brush preview provides essential visual feedback on brush position and size                              |
| `stamp_sampling`          | `StampSampling::Nearest`                               | Nearest-neighbour preserves sharp edges (pixel-art friendly)                                             |
| `stamp_tint_mode`         | `StampTintMode::Original`                              | Use stamp's own colours by default; users opt into tinting                                               |
| `brush_sampling`          | `StampSampling::Nearest`                               | Nearest-neighbour preserves sharp edges                                                                  |
| `brush_tint_mode`         | `StampTintMode::Original`                              | Use brush tip's own colours by default                                                                   |
| `show_grid`               | `false`                                                | Grid is opt-in; hidden by default                                                                        |
| `grid_size`               | `64`                                                   | 64 px provides a useful reference grid without cluttering the canvas                                     |
| `stabilization_enabled`   | `false`                                                | Stabilization is opt-in; disabled by default to preserve the direct 1:1 cursor feel                      |
| `stabilization_smoothing` | `30.0`                                                 | 30.0 provides a gentle smoothing that's noticeable but not sluggish                                      |

### Why manual `Default` instead of derive

The `Color32` type's default is transparent black (`rgba(0, 0, 0, 0)`), which would make the first brush stroke invisible. The `radius` of 0 (default for `u32`) would create a zero-size brush that effectively does nothing. A manual `Default` implementation ensures the application starts in a usable state.
