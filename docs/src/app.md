# app

Top-level application constants, export-format registry, UI state, GPU texture
management, and the main `MyApp` struct that wires together the document, tool
configuration, undo history, and file-IO subsystems for eframe.

## Constants

### App Identity

Three reverse-domain constants identify the application to the OS for platform
data-directory resolution via `directories::ProjectDirs`:

| Constant | Value |
|---|---|
| `APP_QUALIFIER` | `"com"` |
| `APP_ORGANIZATION` | `"Monbuticloud"` |
| `APP_NAME` | `"SplatterIron"` |

`ProjectDirs::from("com", "Monbuticloud", "SplatterIron")` resolves to
a platform-specific path such as `~/.local/share/SplatterIron` on Linux or
`~/Library/Application Support/com.Monbuticloud.SplatterIron` on macOS.

### Canvas File-Format Constants

| Constant | Value | Purpose |
|---|---|---|
| `CANVAS_EXTENSION` | `".splattercanvas"` | Extension for native canvas files (zstd-compressed JSON) |
| `FILE_FILTER_NAME` | `"SplatterCanvas"` | File-dialog filter label displayed in OS pickers |
| `DEFAULT_CANVAS_NAME` | `"canvas.splattercanvas"` | Default save-file name when no path has been set |

### Import Extensions (`IMPORT_EXTENSIONS`)

A flat list of 19 file extensions accepted by the image-import dialog:

`avif`, `png`, `jpg`, `jpeg`, `webp`, `gif`, `tiff`, `tif`, `tga`, `ico`,
`pnm`, `pgm`, `ppm`, `pbm`, `pam`, `qoi`, `exr`, `hdr`, `ff`

These cover all raster image formats supported by the `image` crate, including
legacy formats (TGA, ICO, PNM variants) and HDR/EXR for high-dynamic-range
workflows. The list is used to build the file-type filter shown in native OS
file-open dialogs.

## Export Format Registry

### `struct ExportInformation`

Holds a list of file extensions and the corresponding `image::ImageFormat`
enum variant for one export target.

```rust
pub struct ExportInformation {
    pub extensions: &'static [&'static str],
    pub fmt: image::ImageFormat,
}
```

Used as the value type in the `EXPORT_FORMATS` lookup table. The
`extensions` slice drives the file-extension filter in native save dialogs;
`fmt` is passed directly to `image::ImageEncoder` implementations during
export.

### `EXPORT_FORMATS`

A static lookup table mapping display names to `ExportInformation` entries.
All 13 export targets:

| Display name | Extensions | `image::ImageFormat` |
|---|---|---|
| AVIF | `avif` | `Avif` |
| PNG | `png` | `Png` |
| JPEG | `jpg`, `jpeg` | `Jpeg` |
| WebP | `webp` | `WebP` |
| GIF | `gif` | `Gif` |
| TIFF | `tiff`, `tif` | `Tiff` |
| TGA | `tga` | `Tga` |
| ICO | `ico` | `Ico` |
| PNM | `pnm`, `pgm`, `ppm`, `pbm`, `pam` | `Pnm` |
| QOI | `qoi` | `Qoi` |
| EXR | `exr` | `OpenExr` |
| HDR | `hdr` | `Hdr` |
| Farbfeld | `ff` | `Farbfeld` |

The PNM entry covers all five Portable Anymap sub-formats (PBM/PGM/PPM/PAM).
The table drives the export dialog's format picker and is extensible by
adding entries to the slice.

## UI State

### `struct UIState`

Tracks transient UI concerns that don't belong to any domain module:

| Field | Type | Purpose |
|---|---|---|
| `render_state` | `RenderState` | Current rendering cadence — active, idle-throttled, or unfocused-frozen |
| `time_elapsed` | `Duration` | Total wall-clock time since application start |
| `times_autosaved` | `u32` | Number of autosaves performed this session |
| `last_autosave_time` | `Duration` | Wall-clock timestamp of the most recent autosave completion |
| `displayed_error_list` | `Vec<String>` | Error messages shown in the centred error overlay |
| `pending_layer_for_deletion` | `Option<usize>` | Layer index awaiting deletion confirmation, if any |
| `show_new_canvas_dialog` | `bool` | Whether the "New Canvas" size picker is open |
| `new_canvas_width` | `u32` | Width slider value for the new-canvas dialog (pixels, clamped 4–8192) |
| `new_canvas_height` | `u32` | Height slider value for the new-canvas dialog (pixels, clamped 4–8192) |

The `time_elapsed` field drives the 2-minute autosave interval check in the
main frame loop. `displayed_error_list` is populated by `FileIO::poll_*`
methods and drained by the error overlay window. The new-canvas fields are
used by the "New Canvas" dialog modal and reset on dialog close.

### `impl Default for UIState`

Initialises with `IdleThrottled` rendering, zero elapsed time, no autosaves,
no pending layer deletion, dialog closed, and default dimensions of
2000×1500. This matches the "M" preset in the new-canvas dialog.

## GPU Texture

### `struct GpuTexture`

Holds the wgpu resources for partial-upload canvas rendering:

| Field | Type | Purpose |
|---|---|---|
| `texture` | `wgpu::Texture` | The GPU-side RGBA texture storing the composite canvas image |
| `texture_id` | `egui::TextureId` | Egui texture ID registered with the egui_wgpu renderer for display |
| `queue` | `Arc<wgpu::Queue>` | WGPU command queue for uploading dirty-rect data each frame |

Created during `MyApp::new` when the wgpu backend is available; absent under
the Glow (OpenGL) backend where the egui-managed texture path (full-buffer
`tex.set()`) is used instead.

Each frame, `Document::upload_to_gpu` writes only the dirty sub-region via
`wgpu::Queue::write_texture`, avoiding a full-buffer transfer.

## `struct MyApp`

The top-level application struct owned by eframe, composing every subsystem:

| Field | Type | Purpose |
|---|---|---|
| `document` | `Document` | Canvas document — layers, dimensions, save path |
| `tool_configuration` | `ToolConfiguration` | Active tool, colour, radius, brush-preview toggle |
| `undo` | `UndoHistory` | Undo/redo stack with 1000-entry capacity and visited-stamp deduplication |
| `file_io` | `FileIO` | Async file-dialog and save-operation manager (mpsc channels) |
| `ui` | `UIState` | Render state, autosave counters, dialog flags |
| `gpu_texture` | `Option<GpuTexture>` | WGPU texture for partial-upload rendering; `None` under Glow backend |

All fields are `pub` to give panel methods (`show_top_panel`, `show_left_panel`,
etc.) direct access without getter boilerplate.

## `impl MyApp`

### `MyApp::new(creation_context)`

Constructor invoked once by eframe at startup.

Steps:

1. Creates a pair of mpsc channels — one for dialog-send requests, one for
   save-result notifications.
2. Builds a default `Canvas` (800×600) and computes its pixel count for the
   undo-history capacity.
3. Resolves the platform data directory via `ProjectDirs` using the three
   identity constants, then creates the `autosaves/` subdirectory.
4. When the wgpu render state is available (`creation_context.wgpu_render_state`),
   creates a GPU `Rgba8UnormSrgb` texture sized to the canvas, registers it
   with the egui_wgpu renderer as a native texture via
   `renderer.register_native_texture`, and wraps it in `GpuTexture`.
5. Assembles `MyApp` with a new `Document`, default `ToolConfiguration`,
   `UndoHistory` sized to `pixel_count`, `FileIO` wired to both mpsc
   channels, default `UIState`, and the optional `GpuTexture`.

# Panics

Panics if `ProjectDirs::from` returns `None` (no home directory) or if
`std::fs::create_dir_all` fails for either the data directory or the
autosaves subdirectory (permissions, read-only filesystem).
