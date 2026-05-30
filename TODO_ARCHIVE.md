# TODO Archive

Archived decisions from the TODO pipeline — items that were denied, implemented, or became outdated. All same format as TODO.md

## Denied

- `ActiveWake` render state at unlimited FPS — `src/canvas.rs:370` : already implemented — `ActiveWake(Duration)` with decrement-to-throttle in `src/app/frame.rs:122-129`. (P3)(B1)(59653a1)
- `src/files.rs:211` missing `# Errors` — `ExportStrategy` trait already documents `# Errors` at `src/files.rs:193-195`; impl inherits. (P2)(B1)(59653a1)

## Implemented

- `export_as_image` pixel-by-pixel loop — `src/files.rs:247` → replaced with `bytemuck` cast + rayon `par_chunks_mut` (P1)(B1)(aef7235)(b049292)
- `canvas.rs:262` + `file_io.rs:432` — `output_rgba: Vec<u8>` cloned (12MB) on every export → `Arc<Vec<u8>>` for atomic-shared export. (P1)(B1)(514450e)(d72467d)
- `tools/stamp_brush.rs:148` — `src_x_map` allocated per stamp placement in `stamp_at` → reuse `scratch_src_x` buffer across stamps within a line. (P1)(B1)(514450e)(95feb79)
- `files.rs:262,325,367,387` — export allocates intermediate RgbaImage → skip it for JPEG/HDR/Farbfeld, encode from `raw_output` directly. (P2)(B1)(514450e)(b5ff2ca)

## Outdated

-
