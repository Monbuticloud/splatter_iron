# TODO Archive

Archived decisions from the TODO pipeline — items that were denied, implemented, or became outdated. All same format as TODO.md

## Denied

- `ActiveWake` render state at unlimited FPS — `src/canvas.rs:370` : already implemented — `ActiveWake(Duration)` with decrement-to-throttle in `src/app/frame.rs:122-129`. (P3)(B1)(59653a1)
- `src/files.rs:211` missing `# Errors` — `ExportStrategy` trait already documents `# Errors` at `src/files.rs:193-195`; impl inherits. (P2)(B1)(59653a1)

## Implemented

- `export_as_image` pixel-by-pixel loop — `src/files.rs:247` → replaced with `bytemuck` cast + rayon `par_chunks_mut` (P1)(B1)(aef7235)(b049292)

## Outdated

-
