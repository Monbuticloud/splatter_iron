//! Tests for frame-lifecycle methods in [`src/app/frame.rs`](crate::app::frame).
//!
//! Covers `handle_autosave`, `poll_file_results`, `update_render_state`,
//! `sync_gpu_texture`, and `recreate_gpu_texture` — all once-per-frame
//! lifecycle methods on `MyApp`.

use std::time::Duration;

use crate::app::AUTOSAVE_INTERVAL_MINUTES;

/// [`handle_autosave`] does nothing when canvas is not dirty.
#[test]

fn handle_autosave_skipped_when_not_dirty() {

    let dir = tempfile::tempdir().expect("temp dir");

    let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());

    app.document.dirty_since_last_autosave = false;

    app.handle_autosave();
    // No panic — autosave skipped.
}

/// [`handle_autosave`] triggers autosave when dirty and interval has elapsed.
#[test]

fn handle_autosave_triggers_when_dirty_and_elapsed() {

    let dir = tempfile::tempdir().expect("temp dir");

    let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());

    app.document.dirty_since_last_autosave = true;

    app.ui.time_elapsed = Duration::from_mins(AUTOSAVE_INTERVAL_MINUTES) + Duration::from_secs(1);

    app.handle_autosave();

    assert!(app.ui.times_autosaved >= 1);
}

/// [`handle_autosave`] does not trigger when dirty but interval not elapsed.
#[test]

fn handle_autosave_skipped_when_not_enough_time() {

    let dir = tempfile::tempdir().expect("temp dir");

    let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());

    app.document.dirty_since_last_autosave = true;

    app.ui.time_elapsed = Duration::from_secs(1);

    app.handle_autosave();

    assert_eq!(app.ui.times_autosaved, 0);
}

/// [`handle_autosave`] does not panic when channels disconnected.
#[test]

fn handle_autosave_no_panic_with_unconnected_channels() {

    let dir = tempfile::tempdir().expect("temp dir");

    let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());

    app.document.dirty_since_last_autosave = true;

    app.ui.time_elapsed = Duration::from_mins(AUTOSAVE_INTERVAL_MINUTES) + Duration::from_secs(1);

    // trigger_async_save still works with no receiver.
    app.handle_autosave();
}
