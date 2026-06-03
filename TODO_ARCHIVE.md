# TODO Archive

Archived decisions from the TODO pipeline — items that were denied, implemented, or became outdated. All same format as TODO.md

## Denied

- reduce undo zstd compression level from default to 0 — `undo_history.rs` already uses level `-1` (fast mode), which is faster than level 0 (default/3). No change needed. (P1)(B1)(460008e)(HEAD)

- `#[allow(clippy::..)]` missing inline justification — `src/ui/center.rs:296`, `src/tools/stamp_brush.rs:85` — both now have inline justification comments. (P1)(B1)(59653a1)(5d89e87)
- `unwrap()` calls without justification — `src/ui/dialogs.rs:224,411` — lines no longer contain bare `unwrap()`. (P1)(B1)(59653a1)(5d89e87)
- misplaced docstring — `src/app/mod.rs:75-78` — docstrings correctly attributed per current code. (P2)(B1)(59653a1)(5d89e87)
- duplicate docstring — `src/ui/center.rs:40-46` — no duplication in current codebase. (P3)(B1)(59653a1)(5d89e87)
- missing test module — `src/persistence.rs` has no `src/tests/persistence.rs` — module exists and is registered. (P1)(B1)(59653a1)(5d89e87)
- missing `docs/src/tests/` files — `asset_library.md`, `brush_common.md`, `brush_params.md`, `debug.md` — all 4 files present. (P3)(B0)(59653a1)(5d89e87)
- `ActiveWake` render state at unlimited FPS — `src/canvas.rs:370` : already implemented — `ActiveWake(Duration)` with decrement-to-throttle in `src/app/frame.rs:122-129`. (P3)(B1)(59653a1)
- `src/files.rs:211` missing `# Errors` — `ExportStrategy` trait already documents `# Errors` at `src/files.rs:193-195`; impl inherits. (P2)(B1)(59653a1)
- `ui/top.rs:149` — clone `recent_files` every frame → borrow instead. (P2)(B1)(514450e) — clone is inside `context_menu` closure, only runs on menu open not every frame.
- `document.rs:114-119` — `.collect()` allocates layer ref vec every blend frame → reuse Vec via `clear()`+`push()`. (P1)(B1)(514450e) — allocation is ~90 bytes for 10 layers, negligible.
- `tools/brush_common.rs:53-54` — `apply_visited_runs` returns new `Vec<RunSegment>` per stroke → take `&mut` scratch param. (P1)(B1)(514450e) — `UndoRecord` must own the runs Vec; `std::mem::take` loses scratch capacity, making reuse equivalent to current code.
- `tools/square_brush.rs:189,197` — per-row `Vec::with_capacity` in `draw_square` → scratch `&mut Vec<RunSegment>`. (P1)(B1)(514450e) — same ownership issue; `before` Vecs per row consumed by `compress_run`.
- `tools/circle_brush.rs:139,162,179` — per-row `Vec::new()`/`Vec::with_capacity` in `draw_circle` → scratch `&mut Vec<RunSegment>`. (P1)(B1)(514450e) — same ownership issue.
- `tools/bucket_fill.rs:60-61,86` — runs+stack+per-span allocations in `draw_bucket_fill` → scratch `&mut Vec`. (P1)(B1)(514450e) — same ownership issue.
- `tools/custom_brush.rs:79` — `all_runs` accumulator per stroke → scratch `&mut Vec<RunSegment>`. (P1)(B1)(514450e) — same ownership issue.
- `tools/stamp_brush.rs:161` — per-row `before` Vec in `stamp_at` → scratch buffer. (P1)(B1)(514450e) — `before` Vec consumed by `compress_run` into undo record, ownership transfer unavoidable.
- `blend_region` recomputes `base_indices` and `suppress_base` per dirty rect call — `src/pixel.rs:607-613` → cache when layers haven't changed; add generation counter. (P2)(B1)(5bf5fc9) — overhead ~100ns per call, not worth caching.
- bucket fill lacks visited-stamp dedup — `src/tools/bucket_fill.rs:34` → inline stamp check (not `apply_visited_runs`; architectural mismatch). (P2)(B2)(5bf5fc9) — bucket fill is single-click (no drag), scanline already O(filled_area); visited-stamp would require O(canvas_area) buffer scan.
- alpha overlay rounding drift on repeated undo/redo — `src/undo.rs:279` → store final after-state in undo record instead of re-applying `alpha_blend` each redo. (P2)(B1)(5bf5fc9) — undo stores exact before-pixels from initial stroke; redo always blends from same base state. No drift occurs regardless of undo/redo cycles.

## Implemented

- README is sparse — add build instructions, feature list, screenshot, contribution guide. (P3)(B0)(aef7235)(a904732)
- `export_as_image` pixel-by-pixel loop — `src/files.rs:247` → replaced with `bytemuck` cast + rayon `par_chunks_mut` (P1)(B1)(aef7235)(b049292)
- `canvas.rs:262` + `file_io.rs:432` — `output_rgba: Vec<u8>` cloned (12MB) on every export → `Arc<Vec<u8>>` for atomic-shared export. (P1)(B1)(514450e)(d72467d)
- `tools/stamp_brush.rs:148` — `src_x_map` allocated per stamp placement in `stamp_at` → reuse `scratch_src_x` buffer across stamps within a line. (P1)(B1)(514450e)(95feb79)
- `files.rs:262,325,367,387` — export allocates intermediate RgbaImage → skip it for JPEG/HDR/Farbfeld, encode from `raw_output` directly. (P2)(B1)(514450e)(b5ff2ca)
- `stamp_circle_positions` uses `f64::sqrt` per row — `src/tools/circle_brush.rs:312` → integer midpoint-circle increment (other circle fns already converted). (P1)(B1)(5d89e87)
- square brush `stamp_line_positions` stamps full (2R)² rect at every Bresenham step with no overlap awareness — `src/tools/square_brush.rs:113-119` → per-row span deduplication to avoid re-stamping overlapping pixels. (P1)(B1)(5bf5fc9)
- `redo_apply` alpha overlay path iterates pixel-by-pixel — `src/undo.rs:275-281` → SIMD-vectorize with `wide::u32x4` (pattern already used in `pixel.rs`). (P2)(B1)(5bf5fc9)
- bucket fill stack grows unbounded for large contiguous fills — `src/tools/bucket_fill.rs:64` → added upper bound or switch to bounded scanline queue. (P2)(B1)(5bf5fc9)
- `compress_run` allocates `Vec<Color32>` before RLE check — `src/undo.rs:52` → defer allocation: `apply_visited_runs` checks uniformity without intermediate Vec. (P2)(B1)(5d89e87)
- grid overlay redraws all lines every frame — `src/ui/center.rs:275-291` → cache shapes keyed on (grid_size, cw, ch). (P3)(B1)(5d89e87)
- `apply_stroke` duplicates `BrushStrokeParams` builder construction — `src/ui/center.rs:644-878` → hoisted `layer`, `radius`, `current_tool`; collapsed builder to 1 line. (P1)(B1)(5d89e87)
- drag accumulator has no max frame limit — `src/undo_history.rs:209-211` → added `MAX_DRAG_FRAMES=5000` with auto-finalize. (P2)(B1)(5bf5fc9)
- memory warning estimate ignores actual layer count — `src/app/mod.rs:78-81` → `estimate_canvas_memory` now takes `layer_count` parameter. (P2)(B1)(5bf5fc9)
- missing `# Errors` doc sections — `src/tools/brush_parsers.rs:72,158` — `parse_gbr` and `parse_abr` now have `# Errors`; remaining 5 private fns exempt per standards. (P2)(B1)(5d89e87)
- unused imports — `src/tools/custom_brush.rs:9` (`Canvas`) removed; `file_io.rs` no longer has raw `Path` import (module was split). (P3)(B1)(5d89e87)
- scalar head/tail blending logic duplicated 3× in `apply_single_layer` — `src/pixel.rs:350-483` → extracted into shared `process_pixel` closure. (P3)(B1)(5bf5fc9)
- stale `compress_run` name in module docstring — `src/undo.rs:3` → function renamed to `compress_and_store`. (P4)(B1)(5bf5fc9)
- missing `src/tests/frame.rs` — `src/app/frame.rs` had inline tests; created dedicated test module per convention, migrated 4 tests. (P2)(B1)(5d89e87)
- split `FileIO` into `DialogManager` + `SaveManager` + `ExportManager` + `ImportManager` — async file IO now fully decoupled into 4 submodules. (P1)(B5)(5d89e87)(24f670e)
- remove dead/commented dependencies from `Cargo.toml`: `tokio`, `rand`, `target`, `type-layout`. (P3)(B1)(24f670e)
- increase `PARALLEL_BLEND_THRESHOLD` from 64 to 256 to reduce rayon overhead on small dirty rects. (P2)(B1)(24f670e)
- add image import dimension limits (max 16384×16384 / 50MP) to prevent OOM from malicious images. (P1)(B1)(24f670e)
- add config file size limit (1MB `Read::take`) for `serde_json::from_reader` to prevent malformed config OOM. (P2)(B1)(24f670e)
- add `Superseded-By: ADR-0024` marker to ADR-0006 and ADR-0021 front matter (flat buffer + zstd partially replaced their storage model). (P2)(B1)(24f670e)
- use capacity-based reallocation for `output_rgba` in `blend_to_output` instead of exact-length check to avoid realloc when capacity suffices. (P2)(B1)(24f670e)
- eliminate intermediate `RgbaImage` allocation for all export formats (PNG, WebP, TIFF, TGA, PNM, QOI) — currently only JPEG/HDR/Farbfeld skip it. (P2)(B1)(24f670e)
- stream JPEG RGB output to avoid intermediate `Vec<u8>` allocation — write unpremultiplied RGB directly via pre-allocated buffer. (P2)(B1)(24f670e)
- use `alpha_blend_simd_four` in alpha-overlay brush paths (`bucket_fill.rs`, `circle_brush.rs`) — currently uses scalar `alpha_blend` per pixel. (P2)(B1)(24f670e)
- `stamp_circle_positions` inner loops — converted to midpoint-circle span filling (like `fill_circle_impl`) instead of pixel-by-pixel iteration per Bresenham step. (P3)(B1)(24f670e)(460008e)

## Outdated

- dead code audit — `#[cfg(test)]` gate or remove 21 dead items in `asset_library`, `canvas`, `files`, `undo_history`. (P3)(B1)(460008e)
