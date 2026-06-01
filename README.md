# SplatterIron

**Lightweight, GPU-accelerated digital painting application** built with Rust.

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
| Bucket fill | `G` | Flood-fill contiguous areas |
| Stamp | `T` | Place images onto canvas (nearest/bilinear scaling, original/tinted) |
| Custom brush | `B` | Draw with imported .gbr, .abr, .brush tips |
| Eyedropper | `I` | Pick color from canvas |
| Pan | `H` | Drag to pan viewport |

### Canvas & Layers

- Multi-layer documents — add, delete, rename, reorder, toggle visibility, adjust opacity
- Layer operations are undoable
- Canvas presets: XS 800×600, S 1280×960, M 2000×1500, L 2560×1920, XL 3200×2400 — plus custom sizes
- Zoom 0.05×–20× centered on cursor; double-click to reset
- Brush preview on hover; configurable pixel grid overlay

### File Support

- **Native format**: `.splattercanvas` — compressed, preserves layers and full document state
- **Archive format**: `.splatterarchive` — higher compression ratio for long-term storage
- **Image import**: 19 formats — AVIF, PNG, JPEG, WebP, GIF, TIFF, TGA, ICO, PNM, QOI, EXR, HDR, Farbfeld
- **Image export**: 13 formats — AVIF, PNG, JPEG, WebP, GIF, TIFF, TGA, ICO, PNM, QOI, EXR, HDR, Farbfeld
- **Autosave** every 2 minutes (both `.splattercanvas` and `.splatterarchive`) to `{data_dir}/autosaves/`
- **Config persistence**: tool settings and recent files saved automatically

### Undo / Redo

- Per-pixel granularity — every stroke is fully reversible
- Adjustable step multiplier (1–100 steps per press)
- Shortcuts: `Cmd+Z` undo, `Cmd+Shift+Z` / `Cmd+Y` redo

### Brush & Stamp Libraries

- Persistent stamp library — images stored and managed from the app
- Persistent custom brush library — brush tips imported from files
- Import GIMP `.gbr` (v1/v2) and Photoshop `.abr` (v6–10) brush files
- Stamp scaling: nearest-neighbour or bilinear
- Stamp tint: original colors or multiply by current brush color

---

## Coming Soon

- **Layer blend modes**: Multiply, Screen, Overlay
- **Pressure sensitivity**
- **Selection tools**: rectangular, lasso, magic-wand
- **Brush radius shortcuts**: `[` / `]` to adjust, `Shift+` for fine increments
- **Fullscreen toggle**: `Cmd+Ctrl+F`
- **Preferences dialog**: default canvas size, autosave interval, theme
- **Line tool**: click-drag straight lines with shift-constrain to 45°
- **Color palette / swatches**: persisted color library
- **Canvas rotation and flip**
- **Panel visibility toggles**: `Tab` to toggle all panels

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

## License

Copyright (C) 2026 Nguyen Hoang Quoc Anh (Mon, Monbuticloud)

SPDX-License-Identifier: AGPL-3.0-only

---

## For Developers

### Architecture

Source layout and design decisions are documented in:

- **`AGENTS.md`** — build commands, linting, code standards, commit conventions, and the contributor checklist
- **`docs/architecture/`** — 18 Architecture Decision Records (ADRs)
- **`docs/src/`** — inline module documentation mirrored from source

### Building

Development profile: `overflow-checks = true`, `incremental = true`, `opt-level = 1`.
Release profile: LTO fat, strip debug, panic=abort, `opt-level = 3`.

```bash
cargo build --release
```

### Contributing

- **Conventional Commits**: `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`, `chore:`
- **Atomic commits**: one logical unit per commit
- **Pre-commit**: run `cargo test && cargo clippy`
- Full contributor guide in [`AGENTS.md`](./AGENTS.md)
