# center

Central canvas panel. Renders the composited texture, handles mouse interaction
(brush strokes, eraser, bucket-fill), and applies strokes to the document.

## `MyApp::show_central_panel`

```rust
pub fn show_central_panel(&mut self, ui: &mut egui::Ui)
```

Entry point for rendering the canvas in egui's `CentralPanel`.

### Behaviour

- If either `self.gpu_texture` (wgpu) or `self.document.canvas.rendered_layers`
  (egui fallback texture) is available, delegates to `handle_canvas_interaction`.
- Otherwise no-op — the panel is empty.

## `MyApp::handle_canvas_interaction`

```rust
fn handle_canvas_interaction(&mut self, ui: &mut egui::Ui)
```

Processes all mouse interaction on the canvas texture. This is the core
interaction loop for painting.

### Canvas rendering

The canvas texture is displayed as an `egui::Image` sized to fit the available
area while maintaining aspect ratio. The image has `Sense::click_and_drag()`
and shows a crosshair cursor on hover.

### Context menu

Right-clicking the canvas opens a context menu with:

| Item          | Behaviour                                                         |
| ------------- | ----------------------------------------------------------------- |
| **Import**    | Queues `PendingFileAction::Import`                                |
| **Export As** | Submenu listing all 13 export formats from `EXPORT_FORMATS`       |
| **Save As**   | Queues `PendingFileAction::Save` and clears the current save path |

### Brush preview

When `tool_configuration.show_brush_preview` is true and the cursor hovers over
the canvas, a preview overlay is rendered:

- **Faint dot**: A small grey dot (radius 2.5 px, 31 % gray) is drawn at the
  **raw** (non-stabilized) cursor position to show where the mouse actually is.
- **Circle/CircleEraser**: A circle stroke at the brush radius centred on the
  (stabilized) cursor position, drawn with `PREVIEW_STROKE_WIDTH` (1.0 px).
- **Square/SquareEraser**: A filled rectangle with 20 % fill alpha
  (`PREVIEW_FILL_ALPHA_FACTOR`) and a 1 px border stroke. The fill colour is
  derived from the current brush colour at reduced alpha.
- **BucketFill**: No preview overlay.

The preview position is computed by mapping the mouse UV coordinates to pixel
coordinates on the canvas, then passing through `stabilized_pixel()` when
stabilization is enabled. The faint dot always tracks the raw mouse position,
while the brush preview follows the virtual (stabilized) cursor.

### Render state management

When the canvas is hovered, `render_state` is set to
`RenderState::ActiveWake(Duration::from_millis(550))`, keeping the render loop
active while the user is interacting. `pending_layer_for_deletion` is cleared
on hover.

### Bucket fill on click

When `CurrentTool::BucketFill` is selected and the canvas is clicked:

1. Convert the pointer position to pixel coordinates `(pixel_x, pixel_y)`.
2. Call `draw_bucket_fill` with the current colour and layer.
3. Push the resulting `UndoRecord` onto the undo stack.
4. Mark `dirty_since_last_autosave = true` and `render_next_frame = true`.

### Drag strokes

When the user drags on the canvas (for any non-BucketFill tool):

1. Convert the pointer position to raw pixel coordinates.
2. When stabilization is enabled, call `stabilized_pixel(raw_x, raw_y, dt)` to
   compute a virtual cursor position using a framerate-independent exponential
   ease. On the first drag frame the virtual cursor snaps to the real position;
   on subsequent frames it lerps toward it.
3. Call `apply_stroke(stabilized_x, stabilized_y)` with the (possibly stabilized)
   coordinates.
4. If a stroke was applied:
   - **First frame** (`previous_cursor_position` is `None`): Initialise the
     drag accumulator in `UndoHistory` via `init_drag_accumulator`, then extend
     it with the stroke's runs.
   - **Subsequent frames**: Extend the accumulator with each new stroke's runs.
5. Store the current **stabilized** `(stab_x, stab_y)` in
   `previous_cursor_position` (so the next frame's line interpolation uses
   virtual→virtual, not virtual→real).

When the drag ends (`response.dragged()` is false), the drag accumulator is
finalised via `UndoHistory::finalize_drag_accumulator` and the previous-position
and stabilized-cursor state is reset.

## `MyApp::stabilized_pixel`

```rust
fn stabilized_pixel(&mut self, raw_x: u32, raw_y: u32, dt: f32) -> (u32, u32)
```

Computes a virtual cursor position by lerping toward the real cursor each
frame using a framerate-independent exponential ease. Called during both
brush preview and drag strokes to produce smoothed cursor coordinates.

### Behaviour

- **Stabilization disabled** (`stabilization_enabled == false`): Returns `(raw_x, raw_y)` unchanged.
- **First frame of drag** (`previous_cursor_position.is_none()`): Snaps the
  virtual cursor to the real position so strokes start immediately with no
  delay. Also initialises `stabilized_cursor` if it was `None`.
- **Subsequent frames**: Computes `lerp_factor = 1.0 - exp(-rate * dt)` where
  `rate = 100.0 * max(0.01, 1.0 - smoothing/100.0)`, then updates the virtual
  position: `virtual += lerp_factor * (real - virtual)`.

The `stabilized_cursor` field is reset to `None` when the drag ends (in the
`else` branch of the drag handler).

## `MyApp::apply_stroke`

```rust
fn apply_stroke(&mut self, pixel_x: u32, pixel_y: u32) -> Option<UndoRecord>
```

Applies the current drawing tool at the given pixel coordinates. Returns
`Some(UndoRecord)` for tools that produce strokes, or `None` for
`CurrentTool::BucketFill` (handled via click instead).

### Eraser handling

Erasers (`SquareEraser`, `CircleEraser`) use `Color32::TRANSPARENT` as the
stroke colour with `alpha_overlay` forced to `false`. This replaces pixels
with transparent black (zero alpha).

### Tool dispatch

| Tool                      | First frame (stamp)                                                                                                                                                                                                      | Subsequent frames (line)                                   |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------- |
| `Square` / `SquareEraser` | Calls `draw_square` with `[cx − r, cx + r] × [cy − r, cy + r]` when alpha overlay is off. When alpha overlay is on, calls `draw_square_line` with identical start/end (zero-length line) using the drag stamp machinery. | Calls `draw_square_line` from previous to current position |
| `Circle` / `CircleEraser` | Calls `draw_circle` when alpha overlay is off. When alpha overlay is on, calls `draw_circle_line` with identical start/end using drag stamp machinery.                                                                   | Calls `draw_circle_line` from previous to current position |
| `BucketFill`              | Returns `None`                                                                                                                                                                                                           | Never called (guarded by `handle_canvas_interaction`)      |

### Drag stamp management

When alpha overlay is active and a new drag begins (first frame),
`undo.advance_drag_stamp()` is called to give this drag a unique stamp value.
This ensures that pixels blended in earlier segments of the same drag are not
blended again by later overlapping segments.

## `Context menu: Replace Stamp Image`

When the Stamp tool is active, right-clicking the canvas shows a Replace Stamp Image option that queues PendingFileAction::LoadStamp.

## `Context menu: Replace Brush`

When the CustomBrush tool is active, right-clicking the canvas shows a Replace Brush option that queues PendingFileAction::LoadBrush.

## `Brush preview: Stamp`

When tool is Stamp and show_brush_preview is true, the preview renders the actual stamp image scaled to the brush radius with a border stroke.

## `Brush preview: CustomBrush`

When tool is CustomBrush and show_brush_preview is true, the preview renders the actual brush tip image scaled to the brush radius with a border stroke.

## `Canvas border`

The canvas area is rendered with a dashed purple border (Stroke::new(2.0, Color32::from_rgb(128, 0, 128))) to visually distinguish the canvas from the background.

## `Tool dispatch: Stamp`

First frame: calls draw_stamp_line with identical start/end (single stamp). Subsequent frames: calls draw_stamp_line from previous to current position with interpolation.

## `Tool dispatch: CustomBrush`

First frame: calls draw_custom_brush_line with identical start/end (single tip). Subsequent frames: calls draw_custom_brush_line from previous to current position with spacing-based interpolation.
