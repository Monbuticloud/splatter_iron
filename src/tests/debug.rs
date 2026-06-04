//! Tests for the [`debug_snapshot`] utility.
//!
//! Only compiled when `feature = "debug-snapshot"` is enabled.
//! Verifies the function compiles.  We cannot call `debug_snapshot` in
//! a test because it requires a `&MyApp` reference that cannot be
//! constructed without an eframe context.

/// Ensure the module is accessible.
#[test]

fn module_is_accessible() {

    let _ = crate::debug::debug_snapshot;
}
