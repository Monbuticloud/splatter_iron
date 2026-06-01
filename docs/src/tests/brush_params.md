# tests::brush_params

Tests for the `BrushStrokeParams` parameter bundle — construction, builder
pattern, field access, and debug formatting.

## Test strategy

- Direct field construction and verification.
- Builder with defaults and with `alpha_overlay` override.
- Debug output contains key fields.

## `construction_and_field_access`

Constructs `BrushStrokeParams` directly and asserts all fields match.
