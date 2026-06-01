# ADR 21: Extended Undo Model

- **Status:** Accepted
- **Date:** 2026-06-01

## Context

ADR 6 defined a single `UndoRecord::Run` variant that stores per-pixel
before/after state for brush strokes. This was sufficient when the only
undoable operations were drawing and erasing.

Once the application gained layer-structural operations (add, delete,
reorder, rename, visibility toggle, opacity change), each of those
operations also needed undo support. The single-variant enum could not
represent these operations — they have nothing to do with per-pixel runs.

## Decision

Extend `UndoRecord` to 5 variants, covering both pixel-level and
layer-structural changes:

### UndoRecord (5 variants)

```rust
pub enum UndoRecord {
    /// Per-pixel before/after state for a brush stroke.
    Run {
        layer_index: usize,
        color_after: Color32,
        runs: Vec<RunSegment>,
        is_alpha_overlay: bool,
    },
    /// A new layer was created.
    AddLayer {
        index: usize,
        layer: Box<Layer>,
    },
    /// An existing layer was deleted.
    DeleteLayer {
        index: usize,
        layer: Box<Layer>,
    },
    /// A layer was moved up or down in the stack.
    MoveLayer {
        from_index: usize,
        to_index: usize,
    },
    /// Layer properties (visibility, opacity, name) changed.
    ModifyLayer {
        index: usize,
        old_visible: bool,
        old_opacity: u8,
        old_name: String,
        new_visible: bool,
        new_opacity: u8,
        new_name: String,
    },
}
```

### Apply functions

`undo_apply` and `redo_apply` in `src/undo.rs` handle each variant:

| Variant | Undo | Redo |
|---|---|---|
| `Run` | Restore before-pixels from run segments | Re-fill with `color_after` (or alpha-blend) |
| `AddLayer` | Remove layer at index | Re-insert layer at index |
| `DeleteLayer` | Re-insert layer at index | Remove layer at index |
| `MoveLayer` | Swap back via `swap(from, to)` | Swap via `swap(to, from)` |
| `ModifyLayer` | Restore old properties | Apply new properties |

### Design invariants

- `Box<Layer>` avoids embedding a full `Layer` inline (layer pixel buffers
  can be large; boxing keeps `UndoRecord` small for the common `Run` case).
- `MoveLayer` stores only indices — the layer data itself is unchanged.
- `ModifyLayer` captures both old and new values, making it symmetric for
  undo/redo (no read-from-canvas during redo).

## Consequences

- **Positive:** Every layer operation in the UI (add, delete, reorder,
  rename, visibility, opacity) is now undoable with a single stack.
- **Positive:** The `Run` variant remains unchanged from ADR 6 — all
  existing tool functions continue to return the same variant.
- **Positive:** Layer-structural undo records are small (`AddLayer`/
  `DeleteLayer` hold a `Box<Layer>`, `MoveLayer` holds two `usize` fields).
- **Negative:** The 5-variant enum requires exhaustive matching in
  `undo_apply`/`redo_apply` — adding a new variant touches both apply
  functions and `UndoHistory`'s push logic.
- **Negative:** `ModifyLayer` duplicates old/new property fields (6 fields
  for what could be a diff), increasing memory per record.
- **Negative:** `MoveLayer`'s undo is a simple `swap` only when no other
  structural changes occurred between undo and redo — concurrent layer
  additions could shift indices and corrupt the record.
