# ADR 18: Shared Brush-Stroke Parameter Bundle

- **Status:** Accepted
- **Date:** 2026-05-30

## Context

Every `draw_*_line` function in `src/tools/` (circle, square, custom brush,
stamp brush) accepts a common set of parameters: start/end coordinates,
mutable canvas reference, colour, layer index, visited-stamp buffer, stamp
value, alpha-overlay flag, drag-processed buffer, and drag-stamp value.

This repeated parameter list has several problems:

- **Long signatures**: `draw_circle_line` had 11 parameters; `draw_square_line`
  had 10; `draw_custom_brush_line` had 13. Reading and maintaining these
  signatures is error-prone.
- **Call-site fragility**: Callers must pass arguments in the exact order.
  Adding a new common parameter (e.g. a `pressure` field) requires updating
  every function signature and every call site.
- **Hidden coupling**: The fact that these parameters are the same across all
  tools is not captured in the type system — changing one function's signature
  silently diverges from the others.

## Decision

Introduce `BrushStrokeParams` as a named struct bundling all common parameters:

```rust
pub struct BrushStrokeParams<'a> {
    pub start_x: u32,
    pub start_y: u32,
    pub end_x: u32,
    pub end_y: u32,
    pub canvas: &'a mut Canvas,
    pub color: Color32,
    pub layer: usize,
    pub visited: &'a mut [u32],
    pub stamp: u32,
    pub alpha_overlay: bool,
    pub drag_processed: &'a mut [u32],
    pub drag_stamp_value: u32,
}
```

Every `draw_*_line` function now takes `params: BrushStrokeParams` as the
first argument, followed by tool-specific parameters (radius, spacing, stamp
pixels, sampling, tinting, etc.).

### Key design choices

1. **Struct over tuple**: A named struct documents each field at both the
   definition site and the construction site (`params.start_x = 1`). A tuple
   `(u32, u32, …)` would preserve the ordering problem.

2. **All fields `pub`**: The struct is a pure parameter bundle with no
   invariants or methods (aside from `Debug`). Making fields `pub` avoids
   getter boilerplate without sacrificing encapsulation (there is none).

3. **Single lifetime `'a`**: One lifetime parameter covers both the canvas
   and the two slice references (`visited`, `drag_processed`), which always
   share the same scope in practice. This avoids unnecessary lifetime
   complexity.

4. **Excluded from bundle**: Tool-specific parameters (radius, spacing,
   stamp pixels, tint mode, sampling mode) are NOT included in the bundle.
   These vary independently per tool and putting them in would turn the
   bundle into a "god struct" containing every possible tool's state.

## Consequences

- **Positive:** Tool function signatures are ~60% shorter — the 12 common
  fields collapse into one struct parameter.
- **Positive:** Adding a new common parameter (e.g. `pressure: f32` for
  pressure-sensitive tablets) requires changing only `BrushStrokeParams`
  and the construction site — every tool function automatically gets the
  new field.
- **Positive:** Call-site readability improves — field labels make it clear
  what each value represents.
- **Positive:** A single `#[derive(Debug)]` on the struct gives all tools
  debug-printable parameters.
- **Negative:** Callers must construct a `BrushStrokeParams` instance before
  every tool call, adding a few lines of boilerplate at each call site.
- **Negative:** Borrow checking with struct destructuring can be slightly
  more verbose — all three `&mut` references (`canvas`, `visited`,
  `drag_processed`) must be borrowed simultaneously.
- **Negative:** The struct is defined in a separate module (`brush_params`),
  so adding a new common field requires touching an additional file.
