# ADR 12: wgpu GPU Texture with Partial Upload

- **Status:** Accepted
- **Date:** 2026-05-19
- **Commits:** `cd68d75`, `f442f1d`, `e7b0e16`

## Context

Egui manages textures via `egui::TextureHandle`, which supports `tex.set()`
to replace the entire texture contents. For a 2000×1500 canvas (12 MB RGBA),
uploading the full texture every frame is expensive — even with dirty-rect
blending (ADR-0005), the full pixel buffer still goes to the GPU each frame.

The `egui_wgpu` renderer supports registering native wgpu textures via
`renderer.register_native_texture()`, which allows direct
`wgpu::Queue::write_texture` calls to upload only a sub-region of the texture.

## Decision

Create a `GpuTexture` struct that wraps a wgpu texture, its egui texture ID,
and a queue for partial uploads:

```rust
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub texture_id: egui::TextureId,
    pub queue: Arc<wgpu::Queue>,
}
```

### Dual rendering path

```rust
if self.gpu_texture.is_some() {
    // wgpu path: blend_to_output + upload_to_gpu(only dirty rect)
    let dirty = self.document.blend_to_output();
    self.document.upload_to_gpu(&gpu.queue, &gpu.texture, &dirty);
} else {
    // fallback path (Glow backend): full texture set()
    self.document.render_to_texture(ui);
}
```

### `upload_to_gpu` with sub-region

```rust
pub fn upload_to_gpu(&self, queue: &wgpu::Queue, texture: &wgpu::Texture,
    dirty: &Option<(u32, u32, u32, u32)>) {
    let (x, y, width, height) = dirty.unwrap_or((0, 0, canvas_width, canvas_height));
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture, origin: wgpu::Origin3d { x, y, z: 0 }, ... },
        &self.canvas.output_rgba,
        wgpu::TexelCopyBufferLayout {
            offset: (y * canvas_width + x) * 4,   // row-major offset
            bytes_per_row: Some(canvas_width * 4), // full row stride
        },
        wgpu::Extent3d { width, height, ... },
    );
}
```

### Canvas resize handling

On canvas resize, `recreate_gpu_texture()` destroys the old wgpu texture and
creates a new one with the updated dimensions. The egui texture ID is preserved
via `renderer.update_egui_texture_from_wgpu_texture()` which updates the
existing entry rather than registering a new one.

### Why not always use the wgpu path?

The wgpu render state is only available when `eframe` is running with the wgpu
backend. On systems using the Glow (OpenGL) backend, `wgpu_render_state` is
`None`, and the fallback `render_to_texture()` path using `tex.set()` is used.

## Consequences

- **Positive:** Partial GPU upload reduces bandwidth from 12 MB (full canvas)
  to ~tens of KB (dirty rect) per frame — critical for high-resolution canvases
  at 60 fps.
- **Positive:** The wgpu path avoids the `egui::ColorImage` conversion step
  (`Vec<u8>` → `ColorImage` → texture upload), saving a copy.
- **Positive:** Fallback path ensures the app works on OpenGL-only systems
  without wgpu.
- **Negative:** Two rendering paths to maintain and test (wgpu + fallback).
- **Negative:** `recreate_gpu_texture()` requires holding `render_state.renderer.write()`
  which can panic on lock contention.
- **Negative:** The `offset` calculation in `write_texture` relies on the
  canvas row stride matching the texture row stride (always true for
  `Rgba8UnormSrgb` but must be kept in sync).
