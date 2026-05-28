# ADR 2: Egui/Eframe GUI Framework

- **Status:** Accepted
- **Date:** 2026-05-16
- **Commit:** `0de9592`

## Context

SplatterIron is a cross-platform paint program targeting Windows, macOS, and
Linux. The GUI framework must satisfy:

- **Immediate-mode rendering** — canvas repaints every frame during brush
  strokes; retained-mode overhead (widget tree diffing, invalidation) would add
  latency.
- **GPU-accelerated canvas** — the canvas texture must be rendered as a GPU
  image with minimal copies; the framework must integrate with wgpu for
  dirty-rect partial uploads.
- **Cross-platform native look** — the app should feel native on all three
  desktop platforms without platform-specific UI code.
- **Rich 2D painting primitives** — circles, rectangles, text, images, and
  custom widget shapes are needed for brush preview, UI panels, and the canvas
  itself.
- **Accessibility** — keyboard navigation, screen reader support, and OS theme
  detection out of the box.
- **Lightweight embeddability** — no heavy runtime or virtual DOM; the paint
  program's performance budget should go to pixel blending, not UI overhead.

## Decision

Use **`egui`** (immediate-mode GUI library) with **`eframe`** (native windowing
backend) as the application framework.

```rust
fn main() -> eframe::Result {
    eframe::run_native(
        "SplatterIron",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(MyApp::default())))
    )
}
```

The alternatives considered were:
- **Gtk-rs / relm**: Retained-mode widget tree; canvas repaint would require
  explicit invalidation; no native wgpu integration.
- **winit + imgui-rs**: More control but no built-in widget library; would need
  to build menus, panels, color pickers, and layer list from scratch.
- **iced**: Retained-mode with an Elm-like architecture; canvas widget requires
  custom `Widget` impl and doesn't integrate with wgpu partial uploads.
- **fltk-rs**: Fast but limited 2D drawing API; no GPU texture support.

## Consequences

- **Positive:** Immediate-mode `ui()` method is called every frame, making
  canvas repaint trivial — set `render_next_frame = true` and the texture
  re-uploads on the next cycle.
- **Positive:** `egui::Panel::top/left/right/central` map directly to the four
  UI regions (toolbar, tool palette, settings, canvas).
- **Positive:** Built-in `egui_wgpu` integration enables `GpuTexture` with
  `wgpu::Queue::write_texture` for partial dirty-rect uploads.
- **Positive:** Cross-platform by default — same codebase runs on Windows,
  macOS, Linux, and web (via `eframe::WebRunner`).
- **Negative:** Immediate-mode means UI state must be stored in the app struct
  (`UIState`, `ToolConfiguration`, etc.) rather than in widget trees.
- **Negative:** Egui's `egui::PaintCallback` for custom GPU rendering (e.g.,
  brush preview with alpha overlay) requires understanding egui's internal
  tesselation and clipping.
