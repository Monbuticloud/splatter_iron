# TODO Archive

Archived decisions from the TODO pipeline — items that were denied, implemented, or became outdated. All same format as TODO.md

## Denied

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

## Implemented

- `export_as_image` pixel-by-pixel loop — `src/files.rs:247` → replaced with `bytemuck` cast + rayon `par_chunks_mut` (P1)(B1)(aef7235)(b049292)
- `canvas.rs:262` + `file_io.rs:432` — `output_rgba: Vec<u8>` cloned (12MB) on every export → `Arc<Vec<u8>>` for atomic-shared export. (P1)(B1)(514450e)(d72467d)
- `tools/stamp_brush.rs:148` — `src_x_map` allocated per stamp placement in `stamp_at` → reuse `scratch_src_x` buffer across stamps within a line. (P1)(B1)(514450e)(95feb79)
- `files.rs:262,325,367,387` — export allocates intermediate RgbaImage → skip it for JPEG/HDR/Farbfeld, encode from `raw_output` directly. (P2)(B1)(514450e)(b5ff2ca)

## Outdated

-
