# app::frame

Frame-lifecycle methods called once per frame from `ui()`.

## Methods

| Method                   | Purpose                                                                         |
| ------------------------ | ------------------------------------------------------------------------------- |
| `poll_file_results()`    | Poll file-dialog and save-result channels; transfer loaded stamp/brush data     |
| `update_render_state()`  | Advance the `RenderState` machine; return true if the frame should be skipped   |
| `sync_gpu_texture()`     | Resize GPU texture if canvas changed, blend layers, upload dirty rect           |
| `recreate_gpu_texture()` | Replace the wgpu texture after canvas resize while keeping the same `TextureId` |
| `handle_autosave()`      | Trigger an async autosave every 2 minutes if the canvas is dirty                |

Each method is `pub(crate)` and is called from the `ui()` entry point in `app/mod.rs`.
