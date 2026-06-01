# tests::brush_params

Tests for the `BrushStrokeParams` parameter bundle — construction, builder
pattern, field access, and debug formatting.

## Test strategy

- Direct field construction and verification.
- Builder with defaults and with `alpha_overlay` override.
- Debug output contains key fields.

## `construction_and_field_access`

Constructs `BrushStrokeParams` directly and asserts all fields match.

## `builder_with_defaults`

Uses the builder pattern with no alpha overlay override; verifies `alpha_overlay` defaults to `false`.

## `builder_with_alpha_overlay`

Builder with `.alpha_overlay(true)`; verifies the field is `true`.

## `debug_output`

Verifies the `Debug` format includes `start_x`, `end_y`, `visited.len`, and `drag_processed.len`.
