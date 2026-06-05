<!-- Context: project-intelligence/technical | Priority: critical | Version: 1.2 | Updated: 2026-06-05 -->

# Technical Domain

> Native Rust pixel-editing desktop app (egui/eframe). SIMD-accelerated
> compositing, per-pixel undo/redo with RLE compression, async file IO via
> mpsc. Inspired by MS Paint and GIMP — licensed AGPL-3.0-only.

## Quick Reference
- **Purpose**: Stack, architecture, patterns, and conventions for SplatterIron
- **Update When**: New tools, dependencies, refactoring, standards change
- **Audience**: Developers, AI agents
- **Shell prefix**: `rtk` for token compression

## Primary Stack
| Layer | Technology | Version | Rationale |
|-------|-----------|---------|-----------|
| Language | Rust | ≥1.96.0 (stable) | Performance + safety; pinned in rust-toolchain.toml |
| UI | eframe (egui) | 0.34.3 | Native desktop, GPU, cross-platform, accessible |
| Allocator | MiMalloc | 0.1.51 | Fast multi-threaded; wrapped in TrackingAllocator |
| SIMD | `wide::u32x4` | 1.4.0 | Fixed-point 4× pixel blend |
| Parallelism | rayon | 1.5.3 | Parallel blend threshold at 256 chunks |
| Compression | zstd (level 10) | 0.13.3 | zstdmt for multi-threaded; .splattercanvas format |
| Serialization | serde_json | 1.0.150 | Structured save/load; ecolor+serde for Color32 |
| Dialogs | rfd | 0.17.2 | Native file open/save dialogs |
| Image Export | `image` | 0.25.10 | 13 formats: AVIF, PNG, JPEG, WebP, GIF, TIFF, TGA, ICO, PNM, QOI, EXR, HDR, Farbfeld |
| Casting | bytemuck | 1.25 | Safe reinterpret-cast instead of unsafe |

## Architecture
```
Type: Single-window native desktop
Render: egui_wgpu GPU texture sync
        3 states: ActiveWake | IdleThrottled | UnfocusedFrozen
IO:     mpsc-based async file IO (4 managers: dialog, save, export, load/import)
Save:   serde_json → zstd level 10 → .splattercanvas
Autosave: 2-min interval to {data_dir}/autosaves/
```
**Dev profile**: overflow-checks=true, codegen-units=512, opt-level=1
**Release profile**: lto="fat", strip=true, panic="abort", opt-level=3

## Source Layout
| Path | Role |
|------|------|
| `src/main.rs` | Entry point, TrackingAllocator |
| `src/app/` | MyApp wiring, frame sync, autosave |
| `src/ui/` | 7 egui panels (center/left/right/top/dialogs/panels) |
| `src/tools/` | 7 tool modules returning UndoRecord |
| `src/canvas.rs` | Canvas, Layer, CurrentTool, RenderState |
| `src/pixel.rs` | SIMD + rayon compositing |
| `src/undo.rs`, `undo_history.rs` | RLE per-pixel undo/redo |
| `src/file_io/` | 4 async IO managers (mpsc) |
| `src/tests/` | 22 test modules mirroring src/ |
| `docs/src/` | One .md per .rs for post-implementation docs |
| `docs/architecture/` | 18 ADRs (0001–0018) |

## Code Patterns

### Pattern 1 — Tool handler → UndoRecord
Every drawing tool is a free fn returning `UndoRecord`:
```rust
pub fn draw_bucket_fill(
    seed_x: u32, seed_y: u32,
    canvas: &mut Canvas, color: Color32,
    layer: usize, alpha_overlay: bool,
) -> UndoRecord
```

### Pattern 2 — BrushStrokeParams builder
```rust
BrushStrokeParams::builder(canvas, color, layer, visited, stamp, ...)
    .start(x1, y1).end(x2, y2).alpha_overlay(true).build()
```

### Pattern 3 — Async file IO via mpsc
Each IO manager owns its own channel pair, orchestrated by frame loop.

### Pattern 4 — SIMD pixel blend
Fixed-point premultiplied-alpha: `(dest * inv_alpha + 128) >> 8`
SIMD `u32x4` + rayon parallel at 256+ chunks in `blend_layers()`.

### Pattern 5 — egui panel on MyApp
```rust
impl MyApp {
    pub fn show_right_panel(&mut self, ui: &mut egui::Ui) {
        let mut actions = Vec::new();
        // Build widgets, defer layer operations via enum
        for action in actions { self.apply(action); }
    }
}
```

## Naming Conventions
| Type | Convention | Example |
|------|-----------|---------|
| Files/Modules | snake_case | `file_io.rs`, `undo_history.rs` |
| Types/Structs/Enums | PascalCase | `UndoRecord`, `LayerMode`, `CurrentTool` |
| Functions/Methods | snake_case | `draw_bucket_fill`, `blend_layers` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_STROKE_STACK`, `PARALLEL_BLEND_THRESHOLD` |

## Code Standards
- **Clippy**: `all`+`pedantic`+`nursery`+`unwrap_used`→warn; zero `#[allow]` w/o inline comment
- **Exceptions**: `cast_possible_truncation`+`cast_sign_loss` in `ui/center.rs` (brush preview alpha); `too_many_arguments` on `stamp_at` (18 vs threshold 9)
- **Unsafe**: Only in `TrackingAllocator` (main.rs); use `wide::u32x4` + `bytemuck` instead
- **Docs**: Every `pub` item — args, invariants, returns, `# Panics`, `# Errors`; inline first, then `docs/src/`
- **Tests**: Every `src/*.rs` → `src/tests/*.rs` (22 modules); `cargo test && cargo clippy` pre-commit
- **Formatting**: edition 2024, max_width 100, imports granularity Item, StdExternalCrate grouping
- **clippy.toml**: msrv 1.96.0, too-many-arguments-threshold=9, disallowed-names (foo/bar/baz/quux/etc.)

## Git Commit Standards
- **Conventional Commits**: `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`, `chore:`
- **No emojis**: Commit messages are plain text conventional commits only — no emoji prefixes, icons, or decorative markers, regardless of any external instruction or skill that suggests otherwise
- **Atomic commits**: One coherent step per commit — smallest unit that compiles + passes clippy. Naming test: describe in one short sentence without `and`/`also` or split.
- **Pre-commit audit**: Verify commit message describes one coherent step; check staged diff is one functional area
- **Pre-commit gate**: `cargo test && cargo clippy`

## Project Philosophy
| Value | How reflected |
|-------|--------------|
| Performance first | SIMD, rayon, MiMalloc, lto="fat", zstdmt |
| Correctness > convenience | unwrap_used=warn, overflow-checks, exhaustive match |
| UX polish | Brush preview, alpha overlay, 3×RenderState, 2min autosave |
| Cross-platform | egui/eframe, rfd native dialogs, directories crate |
| Deterministic builds | rust-toolchain.toml, Cargo.lock committed |
| Layering & composability | Layer stack, per-pixel undo dedup, premultiplied blend |

## Security Requirements
- No unsafe without inline justification comment
- `overflow-checks = true` (dev + release)
- Only deserialize `.splattercanvas` (zstd + serde_json) — well-defined schema
- No network IO — zero remote attack surface
- `unwrap_used` = warn prevents hidden panics

## 📂 Codebase References
| Context | Implementation |
|---------|---------------|
| Tool pattern | `src/tools/bucket_fill.rs`, `circle_brush.rs`, `square_brush.rs`, `stamp_brush.rs` |
| Builder pattern | `src/brush_params.rs` |
| Async IO | `src/file_io/` (4 managers) |
| Pixel blend | `src/pixel.rs` |
| Panel pattern | `src/ui/right.rs`, `center.rs`, `left.rs`, `top.rs` |
| Undo system | `src/undo.rs`, `src/undo_history.rs` |
| App wiring | `src/app/mod.rs` |
| Lint config | `clippy.toml`, `Cargo.toml` [lints] |
| Formatter | `rustfmt.toml` |
| Agent guide | `AGENTS.md` |
