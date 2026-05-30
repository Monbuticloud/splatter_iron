# tool_configuration

## `struct ToolConfiguration`

`ToolConfiguration` holds all mutable tool state for the application: the currently selected drawing tool, brush properties (color, radius, alpha-overlay toggle), transient UI interaction state (cursor position for brush preview, previous tool for eraser toggle-back), and undo/redo behavior settings (step multiplier for fast-scroll).

The struct is a plain data container with no methods except [`Default`]. Ownership is held by `MyApp` in `app.rs`, which passes it to the UI panels for display and mutation.

### Fields

| Field                        | Type                  | Purpose                                                                                                                                                                                                                                                             |
| ---------------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `current_tool`               | `CurrentTool`         | The currently selected drawing tool (`Square`, `Circle`, `SquareEraser`, `CircleEraser`). Determines which drawing function is called on pointer events                                                                                                             |
| `current_color`              | `Color32`             | Color applied by brush strokes. Stored in premultiplied-alpha format to match the pixel buffer representation — no conversion is needed during drawing                                                                                                              |
| `radius`                     | `u32`                 | Brush radius in pixels. Controls the size of square/circle brush stamps and the brush preview overlay. Default 100 gives a visible brush out of the box                                                                                                             |
| `alpha_overlay`              | `bool`                | Whether strokes use alpha-overlay blending instead of opaque overwrite. When true, the brush color is blended over the existing pixel via `alpha_blend`; when false, it replaces the pixel outright                                                                 |
| `previous_tool`              | `Option<CurrentTool>` | The tool that was selected before the current tool. Used by the eraser toggle: when the user switches to an eraser tool, the previous tool is saved here; when they switch away from the eraser, the previous tool is restored. `None` if no previous tool is saved |
| `previous_cursor_position`   | `Option<(u32, u32)>`  | Cursor coordinates from the previous frame, in pixel space. Used to compute the brush preview position and to enable per-frame cursor movement deltas. `None` if no previous frame data exists                                                                      |
| `show_brush_preview`         | `bool`                | Whether to render the brush size preview indicator on the canvas. When true, a semi-transparent outline (square or circle, matching the current tool) is drawn at the cursor position                                                                               |
| `undo_redo_steps_multiplier` | `usize`               | Multiplier applied to undo/redo step count during fast-scroll (e.g., holding Ctrl+Shift+Scroll). A value of 1 means one step per scroll tick; higher values accelerate undoing/redoing through many strokes quickly                                                 |
| `stabilization_enabled`      | `bool`                | Whether brush stabilization (lerped virtual cursor) is active. When enabled, the virtual cursor trails the real cursor with a smooth exponential ease for jitter-free strokes                                                                                        |
| `stabilization_smoothing`    | `f32`                 | Smoothing strength (0.0–100.0). Higher values make the virtual cursor lag further behind the real cursor, producing smoother but more delayed strokes. Default 30.0 gives a gentle smoothing that's noticeable but not sluggish                                      |

### Eraser toggle

The `previous_tool` field implements a common UX pattern: toggling between a drawing tool and an eraser. When the user selects `SquareEraser` or `CircleEraser`, the previously selected tool is stashed in `previous_tool`. When the user deselects the eraser (or switches back), the stashed tool is restored. This means a single keyboard shortcut can toggle between the last drawing tool and the eraser.

### Alpha-overlay vs opaque

The `alpha_overlay` flag affects how the brush interacts with existing pixels:

- **Opaque** (`alpha_overlay: false`): The brush color completely replaces the pixel. This is the default and is suitable for most painting.
- **Alpha overlay** (`alpha_overlay: true`): The brush color is blended over the existing pixel using premultiplied-alpha compositing. This allows the brushed color to be semi-transparent, letting the underlying image show through.

The distinction matters for undo/redo: opaque strokes can be reapplied with a bulk `fill()`, while alpha-overlay strokes require per-pixel blending, which is handled in `redo_apply`.

## `impl Default for ToolConfiguration`

Provides sensible defaults for the initial application state. Rust's `#[derive(Default)]` is not used because several fields require specific values that differ from typical type defaults (e.g., `Color32` defaults to transparent black, but the initial brush color should be opaque white).

### Default values

| Field                        | Default                                                | Rationale                                                                                                                                                              |
| ---------------------------- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `current_tool`               | `CurrentTool::Square`                                  | Square is the simplest and most intuitive default tool for a raster paint application                                                                                  |
| `current_color`              | `Color32::from_rgba_premultiplied(255, 255, 255, 255)` | Opaque white — a neutral starting color that shows clearly against any canvas background                                                                               |
| `radius`                     | `100`                                                  | 100 pixels gives a brush large enough to be immediately usable and visible at typical canvas zoom levels                                                               |
| `alpha_overlay`              | `false`                                                | Opaque painting is the expected default; users opt into alpha blending consciously                                                                                     |
| `previous_tool`              | `None`                                                 | No previous tool exists on startup                                                                                                                                     |
| `previous_cursor_position`   | `None`                                                 | No cursor history exists on startup                                                                                                                                    |
| `show_brush_preview`         | `true`                                                | Brush preview provides essential visual feedback on brush position and size                                                                                            |
| `undo_redo_steps_multiplier` | `1`                                                    | One step per scroll tick provides fine-grained control. Users can scroll multiple ticks to undo/redo several steps, with each tick corresponding to exactly one stroke |
| `stabilization_enabled`      | `false`                                                | Stabilization is opt-in; disabled by default to preserve the direct 1:1 cursor feel                                                                                    |
| `stabilization_smoothing`    | `30.0`                                                 | 30.0 provides a gentle smoothing that's noticeable but not sluggish. Full range is 0.0 (snappy) to 100.0 (frozen)                                                      |

### Why manual `Default` instead of derive

The `Color32` type's default is transparent black (`rgba(0, 0, 0, 0)`), which would make the first brush stroke invisible. The `radius` of 0 (default for `u32`) would create a zero-size brush that effectively does nothing. A manual `Default` implementation ensures the application starts in a usable state.

## `stamp_sampling`

StampSampling strategy when scaling stamp images to canvas size. Defaults to Nearest (sharp edges, pixel-art friendly). Can be set to Bilinear for smooth scaling of photographs.

## `stamp_tint_mode`

StampTintMode controlling whether stamp pixels are multiplied by current_color. Defaults to Original (use stamp's own colours). Can be set to Tinted to apply the current tool colour.

## `brush_sampling`

StampSampling strategy when scaling custom brush tips. Defaults to Nearest.

## `brush_tint_mode`

StampTintMode controlling whether custom brush pixels are tinted by current_color. Defaults to Original.
