# SplatterIron Domain Glossary

## Core Data

- **Canvas** — Owns pixel layers and dimensions. `Vec<Layer>`, `width`, `height`, `output_rgba` buffer, `rendered_layers` texture handle, `dirty_rect` list. Serialized as the body of `.splattercanvas` files.
- **Layer** — A single RGBA pixel array (`Vec<Color32>`, premultiplied). Composited bottom-to-top via `blend_layers`.
- **Document** — Wraps `Arc<Canvas>` with `savefile_path`, `current_layer`, `dirty_since_last_autosave`, and GPU upload methods. Arc enables COW during async saves.

## Rendering

- **DirtyRect** — `{ min_x, min_y, max_x, max_y: u32 }` bounding box of modified pixels. Tool functions extend it per stamp; `blend_region` re-blends only within this box.
- **DirtyRectList** — `Vec<DirtyRect>` with proximity merge (≤16 px gap, max 8 rects). Replaced single `Option<DirtyRect>` to avoid coarse union of distant stroke endpoints.
- **RenderState** — Enforces frame-rate budget: `ActiveWake(duration)` for repaint-after, `IdleThrottled` for delayed refresh, `UnfocusedFrozen` for zero CPU when unfocused.

## Undo System

- **UndoRecord** — One per stroke: `layer_index`, `color_after`, `runs: Vec<RunSegment>`, `is_alpha_overlay`. Single variant `Run`; struct when future variants materialise.
- **RunSegment** — `{ start: u32, length: u32, before: BeforePixels }`. Contiguous pixel run captured before modification.
- **BeforePixels** — RLE: `All(Color32)` for uniform runs >8 px, `Many(Vec<Color32>)` for short or non-uniform runs.
- **VisitedStamp** — Per-pixel `Vec<u32>` counter. Each stroke increments `visited_stamp`; pixels touched by this stroke get `visited[pixel] = stamp`. O(1) dedup prevents re-processing the same pixel within a stroke.
- **DragStamp** — Separate per-pixel `Vec<u32>` for alpha-overlay mode. `drag_stamp_value` advances per drag gesture; prevents alpha-accumulation when overlapping brush segments retouch the same pixel.
- **DragAccumulator** — Collects `RunSegment`s across frames of a drag gesture. Prepends new runs before old so `undo_apply` walks back through correct intermediate states. Finalised into one `UndoRecord` on drag end.
- **UndoHistory** — `VecDeque<UndoRecord>` (max 1000), `redo_index`, visited/drag stamp buffers, drag accumulator. Methods: `push_undo`, `undo_step`/`redo_step`, `next_stamp`, `advance_drag_stamp`, `init/extend/finalize_drag_accumulator`.

## Tools

- **CurrentTool** — `Square`, `Circle`, `SquareEraser`, `CircleEraser`, `BucketFill`, `Stamp`, `CustomBrush`. Drives dispatch in `apply_stroke`.
- **AlphaOverlay** — When true, strokes alpha-blend over destination instead of opaque fill. Correct for soft brushes; prevents harsh edges on partial opacity.
- **BrushStrokeParams** — Parameter object bundling all inputs for line-drawing functions: endpoints, radius, canvas, color, layer, visited/drag buffers, stamp values, tip data, tint/sampling.
- **BucketFill** — Scanline flood-fill. Replaces contiguous same-colour region with `current_color`. Returns `UndoRecord`.
- **Eraser** — `SquareEraser`/`CircleEraser` reuse the same brush geometry as Square/Circle but emit `Color32::TRANSPARENT` with alpha overlay disabled.

## Asset Libraries

- **StampLibrary** — Persistent collection of stamp images. Each `StampEntry`: name, pixels, dimensions, texture handle. Stored as PNG + `index.json` in `{data_dir}/stamps/`.
- **BrushLibrary** — Same as StampLibrary but for custom brush tips. `BrushEntry` adds `spacing`. Stored in `{data_dir}/brushes/`. Imports from `.gbr` and `.abr` files.
- **StampSampling** — `Nearest` or `Bilinear` interpolation when scaling tips to canvas size.
- **StampTintMode** — `Original` (use tip colours) or `Tinted` (multiply by `current_color`).

## Graphics & Performance

- **PremultipliedAlpha** — RGBA format where each RGB channel is pre-multiplied by alpha. Used for all layer pixels and blend operations. Enables correct `source-over` compositing without colour fringing.
- **GpuTexture** — wgpu `Rgba8UnormSrgb` texture + egui `TextureId` + `Arc<Queue>`. Partial upload via `write_texture` with dirty-rect offset. Falls back to egui `tex.set()` under Glow.
- **blend_layers** — SIMD (`wide::u32x4`) + rayon parallel blend of all layers into `output_rgba`. Full-canvas.
- **blend_region** — Row-by-row sequential blend of a dirty rect sub-region. SIMD within each row segment.

## File I/O

- **FileIO** — Two mpsc channels: one for dialog results (path from native file picker), one for save results (completion/error). Polled once per frame at top of `ui()`.
- **SaveKind** — `ManualSave` (explicit user save/export) or `Autosave` (2-minute interval).
- **SaveState** — `Idle` (no save in progress, safe to avoid COW) or `InFlight` (async save holds `Arc<Canvas>`, must COW on mutation).

## UI

- **UIState** — Transient frame-loop state: `RenderState`, autosave counters, dialog flags (`show_new_canvas_dialog`, `pending_large_canvas`), pending import data (`pending_stamp_name`, `pending_brushes`), error list, toast message, cursor position for drag detection.
- **ToolConfiguration** — Persistent tool settings: `current_tool`, `current_color`, `radius`, `alpha_overlay`, `show_brush_preview`, stamp/brush `sampling` and `tint_mode`.
