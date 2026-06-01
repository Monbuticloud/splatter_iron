# app

Top-level application constants, export-format registry, `UIState`, `DialogState`,
`GpuTexture`, and the `MyApp` struct that wires together the document, tools, undo
history, and file-IO subsystems for eframe.

## Submodules

| Module   | Purpose                                                                 |
| -------- | ----------------------------------------------------------------------- |
| `frame`  | Frame-lifecycle methods: poll I/O, render-state machine, GPU sync, autosave |

## Constants, Structs & Enums

All app identity constants (`APP_QUALIFIER`, `APP_ORGANIZATION`, `APP_NAME`),
performance constants (`UNFOCUSED_SLEEP_MILLISECONDS`, `REPAINT_DELAY_MULTIPLIER`,
`AUTOSAVE_INTERVAL_MINUTES`), canvas presets and thresholds (`NEW_CANVAS_PRESETS`,
`MEMORY_WARNING_THRESHOLD`, `estimate_canvas_memory`),
canvas format constants (`CANVAS_EXTENSION`, `FILE_FILTER_NAME`,
`DEFAULT_CANVAS_NAME`), archive format constants (`ARCHIVE_EXTENSION`,
`ARCHIVE_FILTER_NAME`, `DEFAULT_ARCHIVE_NAME`), `IMPORT_EXTENSIONS`, `EXPORT_FORMATS`,
`ExportInformation`, `PersistedConfig`, `PendingStamp`, `DialogState`,
`ErrorState`, `ToastState`, `UnsavedWarningAction`, `ProgressState`, `UIState`,
`GpuTexture`, and `MyApp` are defined here alongside `new()` and the
`impl eframe::App for MyApp` entry point.

## `impl MyApp`

### `MyApp::new(creation_context)`

Constructor invoked once by eframe at startup.

Steps:
1. Creates mpsc channels for file-dialog and save-result communication.
2. Builds a default `Canvas` (2000×1500) and computes pixel count for undo capacity.
3. Resolves the platform data directory via `ProjectDirs`, creates `autosaves/`.
4. Queries `max_texture_dimension_2d` from the wgpu device (falls back to 8192).
5. Loads tool configuration and recent files from `config.json` via `load_config`.
6. Loads `StampLibrary` and `BrushLibrary` from disk.
7. When wgpu is available, creates a GPU `Rgba8UnormSrgb` texture and registers
   it with the egui_wgpu renderer.
7. Assembles `MyApp` with the loaded/created subsystems and optional `GpuTexture`.

### Panics

Panics if `ProjectDirs::from` returns `None` or `create_dir_all` fails for the
data directory or autosaves subdirectory.

## `impl eframe::App for MyApp`

### `fn ui(&mut self, ui, frame)`

The per-frame entry point called by eframe. Orchestrates the frame lifecycle:

1. **Poll async I/O** — calls `self.poll_file_results(ui.ctx())`.
2. **Render state** — calls `self.update_render_state(ui)`. If unfocused, returns early.
3. **GPU sync** — calls `self.sync_gpu_texture(frame, ui)` to resize/blend/upload.
4. **Panels** — calls `self.show_panels(ui)` which renders top/left/right/centre panels.
5. **Dialogs** — calls error window, delete-layer, large-canvas, new-canvas,
   unsaved-changes, stamp-naming, brush-naming, and toast dialogs.
6. **Title** — updates window title to reflect unsaved-changes state.
7. **Close** — sends `ViewportCommand::Close` on quit or `is_quitting`.
8. **Autosave** — calls `self.handle_autosave()` and `self.handle_config_save()`.
