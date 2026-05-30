//! Debug utilities for development builds.
//!
//! Provides `debug_snapshot` which dumps application state via `dbg!`
//! when compiled in debug mode.  No-ops in release builds.

use crate::app::MyApp;
use crate::main;

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
    #[cfg(debug_assertions)]
    {
        dbg!(app);
    }
}
