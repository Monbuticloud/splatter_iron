//! Debug utilities gated behind the `debug-snapshot` feature.
//!
//! Provides `debug_snapshot` which dumps application state via `dbg!`.
//! Only compiled when `feature = "debug-snapshot"` is enabled.

use crate::app::MyApp;

/// Dump a snapshot of the application state using `dbg!`.
///
/// # Parameters
///
/// * `app` — The application state to inspect.
///
/// # Panics
///
/// Panics only via the inner `dbg!` macro (no additional panics).
pub fn debug_snapshot(app: &MyApp) {
    dbg!(app);
}
