# main

Application entry point: installs the MiMalloc global allocator, resolves
the platform data directory, and launches the eframe GUI event loop.

## `fn main() -> eframe::Result`

`main` is the single entry point. It does three things:

1. **Resolve the platform data directory.** Uses `directories::ProjectDirs`
   to find the OS-appropriate location for application data (e.g.
   `~/.local/share/splatter_iron/` on Linux, `~/Library/Application
   Support/splatter_iron/` on macOS). Panics if the home directory cannot be
   found.
2. **Create the data directory.** Calls `create_dir_all` to ensure the path
   exists, creating any missing parents. Panics if the filesystem refuses
   (e.g. permissions).
3. **Launch the GUI.** Calls `eframe::run_native` with the app name, default
   native options, and a closure that constructs `app::MyApp`. Ownership of
   `MyApp` (and all application state — `Document`, `ToolConfig`,
   `UndoHistory`, `FileIO`, `UIState`) is transferred to the egui event
   loop. The call is blocking and returns only when the window is closed.

The function returns `eframe::Result`, which propagates any error from window
creation (missing display server, unsupported OpenGL version, etc.).

## `static GLOBAL: MiMalloc`

`MiMalloc` is the global allocator for the entire process, installed via
`#[global_allocator]`. It replaces the system allocator with Microsoft's
[mimalloc](https://github.com/microsoft/mimalloc), a compact general-purpose
allocator that provides predictable low-latency allocations and good
multi-threaded scaling.

SplatterIron allocates and frees canvas pixel buffers (up to millions of
`Color32` values) on every brush stroke and during undo/redo, so allocator
performance directly impacts frame latency. MiMalloc was chosen over the
system allocator for its measured throughput advantage in this allocation
pattern.

## Module declarations

`src/main.rs` declares 14 modules that make up SplatterIron's crate root:

| Module | Source | Role |
|---|---|---|
| `app` | `src/app.rs` | `MyApp` — top-level UI wiring, `UIState`, async autosave loop |
| `canvas` | `src/canvas.rs` | `Canvas`, `Layer`, `CurrentTool`, `RenderState` |
| `document` | `src/document.rs` | `Document` — canvas + layer stack + save path |
| `file_io` | `src/file_io.rs` | `FileIO` — async file dialogs via mpsc channels |
| `files` | `src/files.rs` | `save_canvas`, `load_canvas`, `export_as_image` — zstd-compressed JSON I/O |
| `pixel` | `src/pixel.rs` | SIMD + rayon premultiplied-alpha pixel blending |
| `tool_configuration` | `src/tool_configuration.rs` | `ToolConfig` — current tool, color, radius, brush preview toggle |
| `tools` | `src/tools/` | Brush engines: `bucket_fill`, `circle_brush`, `square_brush` |
| `ui` | `src/ui/` | 4 egui panels: `top` (menu), `left` (tools), `right` (color/layers), `center` (canvas) |
| `undo` | `src/undo.rs` | `UndoRecord`, per-pixel stroke apply / undo / redo |
| `undo_history` | `src/undo_history.rs` | `UndoHistory` — undo/redo stack with visited-stamp dedup |
| `tests` | `src/tests/` | 9 test modules mirroring `src/` modules |

Each module is gated behind the standard `mod` declaration; the `tests` module
is additionally gated behind `#[cfg(test)]`.
