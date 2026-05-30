//! Tests for the [`debug_snapshot`] utility.
//!
//! Verifies the function compiles.  The actual `dbg!` output is only
//! produced in debug builds; in release builds the function body is a
//! no-op.  We cannot call `debug_snapshot` in a test because it requires
//! a `&MyApp` reference that cannot be constructed without an eframe
//! context.

/// Ensure the module is accessible.
#[test]
fn module_is_accessible() {
    let _ = crate::debug::debug_snapshot;
}
