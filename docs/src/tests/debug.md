# tests::debug

Tests for the `debug_snapshot` utility. Only compiled under the
`debug-snapshot` feature. Verifies the module is accessible.

## Test strategy

- Verify the function compiles and is reachable.

## `module_is_accessible`

Asserts that `crate::debug::debug_snapshot` is a reachable function symbol.
