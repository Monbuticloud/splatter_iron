# SplatterIron

**Lightweight, GPU-accelerated digital painting application** built with Rust, `egui`/`eframe`, and wgpu.

> **MSRV**: Rust 1.96.0 (stable) — pinned in `rust-toolchain.toml`
>
> **License**: AGPL-3.0-only

---

## Features

### Drawing Tools

| Tool | Shortcut | Description |
|------|----------|-------------|
| Square brush | `S` | Filled rectangles |
| Circle brush | `C` | Filled circles |
| Square eraser | `E` | Rectangular erasure |
| Circle eraser | `Shift+E` | Circular erasure |
| Bucket fill | `G` | Scanline flood-fill |
| Stamp | `T` | Place images onto canvas (nearest/bilinear sampling, original/tinted mode) |
| Custom brush | `B` | Draw with imported brush tips (.gbr, .abr, .brush) |
| Eyedropper | `I` | Pick color from canvas |
| Pan | `H` | Drag to pan viewport |

All tools produce per-pixel `UndoRecord` for fully reversible strokes.

### Canvas & Layers

- Multi-layer document with add, delete, rename, reorder, visibility toggle, and per-layer opacity
- Layer operations are undoable (add, delete, move, modify)
- Configurable canvas presets (XS 800×600 through XL 3200×2400) plus custom sizes
- GPU-driven compositing with dirty-rect tracking for efficient partial uploads
- Zoom (0.05×–20×) centered on cursor, double-click to reset
- Brush preview on hover, configurable pixel grid overlay

### Performance

- **SIMD** (`wide::u32x4`) + **rayon** parallel layer blending
- **MiMalloc** global allocator with `TrackingAllocator`
- **Render state machine**: active wake (fast repaints), idle throttled, unfocused frozen
- **Async file I/O** via mpsc channels — saving, loading, and exporting never block the UI
- **Dirty-rect tracking** with proximity-based merging (up to 8 rects before full merge)
- **Release profile**: LTO fat, strip debug, panic=abort

### File Support

- **Native format**: `.splattercanvas` — `serde_json` → `zstd` level 10 (multithreaded)
- **Archive format**: `.splatterarchive` — `xz2` preset 9 compression
- **Image import**: 19 formats (AVIF, PNG, JPEG, WebP, GIF, TIFF, TGA, ICO, PNM, QOI, EXR, HDR, Farbfeld + aliases)
- **Image export**: 13 formats — AVIF, PNG, JPEG, WebP, GIF, TIFF, TGA, ICO, PNM, QOI, EXR, HDR, Farbfeld
- **Autosave** every 2 minutes (both `.splattercanvas` and `.splatterarchive`) to `{data_dir}/autosaves/`
- **Config persistence**: tool settings + recent files (up to 10) saved to `config.json`

### Undo / Redo

- Per-pixel stroke undo/redo with visited-stamp deduplication
- Drag accumulator merges per-frame run segments into one undo record
- Adjustable step multiplier (1–100 steps per press)
- Max stroke stack: 1000 entries
- Shortcuts: `Cmd+Z` undo, `Cmd+Shift+Z` / `Cmd+Y` redo

### Brush & Stamp Libraries

- Persistent stamp library — images stored as PNG with JSON index on disk
- Persistent custom brush library — brush tips stored as PNG with JSON index
- Import GIMP `.gbr` (v1/v2) and Photoshop `.abr` (v6–10) brush files
- ABR parser supports sampled brushes (embedded PNG / raw BGRA) and computed brushes (round, square, diamond, capped-round with hardness)
- Stamp sampling: nearest-neighbour, bilinear
- Stamp tint modes: original, tinted (multiply by current color)

---

## Coming Soon

- **Layer blend modes**: Multiply, Screen, Overlay, and others beyond alpha-overlay
- **Pressure sensitivity**: `pressure: f32` integrated into brush parameters
- **Selection tools**: rectangular, lasso, magic-wand
- **Brush radius shortcuts**: `[` / `]` to adjust, `Shift+` for fine increments
- **Fullscreen toggle**: `Cmd+Ctrl+F`
- **Preferences dialog**: default canvas size, autosave interval, theme
- **Line tool**: click-drag straight lines with shift-constrain to 45°
- **Color palette / swatches**: save, load, recent colors, persisted to disk
- **Canvas rotation and flip**
- **Panel visibility toggles**: `Tab` to toggle all panels, individual show/hide

---

## Benchmarks

Preliminary memory comparison: SplatterIron vs. GIMP at 2000×2000 canvas, single layer, macOS.

| Application | Real Memory | Canvas | Layers | Focused | OS |
|---|---|---|---|---|---|
| SplatterIron | 147.1 MB | 2000×2000 | 1 | No | macOS |
| SplatterIron | 332.7 MB | 2000×2000 | 1 | Yes | macOS |
| GIMP | 337.8 MB | 2000×2000 | 1 | No | macOS |
| GIMP | 355.0 MB | 2000×2000 | 1 | Yes | macOS |

*Single-run measurements. SplatterIron consumes comparable or less memory than GIMP under the same canvas load.*

---

## Quick Start

**Prerequisites**: Rust ≥ 1.96.0 (stable).

```bash
# Build
cargo build

# Run
cargo run

# Test
cargo test

# Lint
cargo clippy
```

Development profile uses `overflow-checks = true`, `incremental = true`, and `opt-level = 1`.

---

## Usage

### Keyboard Shortcuts

**Tool switching** (no modifier): `S` / `C` / `E` (toggle eraser) / `Shift+E` (circle eraser) / `G` / `T` / `B` / `I` / `H`

**File operations** (`Cmd`):
| Shortcut | Action |
|---|---|
| `Cmd+N` | New canvas |
| `Cmd+O` | Open `.splattercanvas` |
| `Cmd+S` | Save |
| `Cmd+Shift+S` | Save As |
| `Cmd+I` | Import image |
| `Cmd+E` | Export (last format) |

**Canvas**: scroll wheel / pinch to zoom, double-click to reset zoom to 100%, drag with Pan tool to pan.

---

## File Format

`.splattercanvas` files contain `serde_json`-serialized canvas data (layers, dimensions, pixel data) compressed with `zstd` at level 10. `.splatterarchive` uses `xz2` at preset 9 for higher compression ratio.

Both formats preserve full layer stack, pixel data, and document metadata.

---

## Architecture

SplatterIron is organized into ~30 source modules:

| Module | Role |
|---|---|
| `src/app/` | Application wiring, GPU texture sync, autosave loop |
| `src/canvas.rs` | Canvas, layer, and tool state |
| `src/document.rs` | Document wrapper — canvas + layer stack + save path |
| `src/pixel.rs` | SIMD + rayon parallel blend, premultiplied alpha |
| `src/undo_history.rs` | Per-pixel undo/redo with visited-stamp dedup |
| `src/tool_configuration.rs` | Tool parameters, color, radius |
| `src/tools/` | 7 tool implementations (square, circle, bucket fill, stamp, custom brush, brush_common, brush_parsers) |
| `src/ui/` | 7 panels: center (canvas), top (menu), left (tools), right (color/layers), dialogs, panels |
| `src/file_io.rs` | Async file dialogs via mpsc channels |
| `src/files.rs` | Save/load/export/import encoders |
| `src/asset_library.rs` | Generic persistent library (brushes, stamps) |
| `src/tests/` | 22 test modules covering all major subsystems |

Architecture decisions are documented in `docs/architecture/` (18 ADRs). Module-level docs are in `docs/src/`.

---

## Contributing

Contributions are welcome. Please follow these conventions:

- **Conventional Commits**: `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`, `chore:`
- **Atomic commits**: each commit contains exactly one logical unit (one function, one test, one doc change, etc.)
- **Pre-commit**: run `cargo test && cargo clippy` before every commit
- **All public items** must have inline docstrings documenting arguments, invariants, return values, errors, and panics

Full contributor guide (including commit checklist, code standards, and testing requirements) is available in [`AGENTS.md`](./AGENTS.md).

---

## License

Copyright (C) 2026 Nguyen Hoang Quoc Anh (Mon, Monbuticloud)

SPDX-License-Identifier: AGPL-3.0-only
