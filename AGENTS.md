# SplatterIron — Agent Guide

## Build & Dev

- **Requires nightly Rust** — `edition = "2024"` + `build-std = ["std"]`.
  Switch: `rustup override set nightly`.
- `.cargo/config.toml` overrides global build flags (`+crt-static`, `opt-level=3`, `debuginfo=0`).
- Build: `cargo build` / Run: `cargo run`. **No tests exist** — no `#[test]` in repo.
- Prefix shell commands with `rtk` for token compression (per `.clinerules`).

## Lints

- Clippy in `Cargo.toml`: `all`, `pedantic`, `nursery`, `unwrap_used` → `warn`.
  Check: `cargo clippy`.
- No `rustfmt.toml` or `clippy.toml` — toolchain defaults used.

## Source Layout

| File | Role |
|------|------|
| `src/main.rs` | Entry point, custom `TrackingAllocator` (MiMalloc), `eframe::run_native` |
| `src/app.rs` | `MyApp` state, async file dialogs/saves, render-to-texture, autosave (2min interval) |
| `src/canvas.rs` | `Canvas`/`Layer`, drawing primitives, `CurrentTool` enum |
| `src/pixel.rs` | SIMD (`wide::u32x4`) + rayon parallel blend, premultiplied-alpha |
| `src/files.rs` | zstd-compressed JSON (`.splattercanvas`), image import/export (17 formats) |
| `src/undo.rs` | Per-pixel stroke undo/redo |
| `src/ui/` | 4 panels: top (file menu), left (tools), right (color/layers), center (canvas) |

## Notable

- File format: `serde_json` → `zstd` level 10 → `.splattercanvas`
- No CI, no Makefile/justfile, no test harness
- Dev profile: `overflow-checks = true`, `opt-level = 1`
- Release: `lto = "fat"`, `strip = true`, `panic = "abort"`
