//! Tests for `BrushLibrary` — add, remove, select, persistence round-trip.
//!
//! Mirrors the `stamp_library` test patterns.

use eframe::egui::Context;

use crate::brush_library::BrushLibrary;
use crate::tests::common::red;

fn tempdir() -> std::path::PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("brush_lib_test_{ts}"));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Add one brush and verify it is selected.
#[test]
fn add_brush_increments_count() {
    let dir = tempdir();
    let mut lib = BrushLibrary::load_from_disk(&dir);

    assert!(lib.is_empty());
    assert_eq!(lib.len(), 0);
    assert!(lib.selected().is_none());

    lib.add("test".to_string(), vec![red(); 4], 2, 2, 25, &Context::default());

    assert_eq!(lib.len(), 1);
    assert!(lib.selected().is_some());
    assert_eq!(lib.selected().unwrap().name, "test");
    assert_eq!(lib.selected().unwrap().width, 2);
    assert_eq!(lib.selected().unwrap().height, 2);
    assert_eq!(lib.selected().unwrap().spacing, 25);
}

/// Remove a brush and verify count decreases and selection clears.
#[test]
fn remove_brush_decrements_count() {
    let dir = tempdir();
    let mut lib = BrushLibrary::load_from_disk(&dir);

    lib.add("to_remove".to_string(), vec![red(); 4], 2, 2, 20, &Context::default());
    assert_eq!(lib.len(), 1);

    lib.remove(0);
    assert!(lib.is_empty());
    assert!(lib.selected().is_none());
}

/// Select a specific brush by index.
#[test]
fn select_switches_active_brush() {
    let dir = tempdir();
    let mut lib = BrushLibrary::load_from_disk(&dir);

    lib.add("first".to_string(), vec![red(); 4], 2, 2, 25, &Context::default());
    lib.add("second".to_string(), vec![red(); 4], 2, 2, 30, &Context::default());

    lib.select(0);
    assert_eq!(lib.selected_index(), Some(0));
    assert_eq!(lib.selected().unwrap().name, "first");

    lib.select(1);
    assert_eq!(lib.selected_index(), Some(1));
    assert_eq!(lib.selected().unwrap().name, "second");
}

/// Persist to temp dir, reload, and verify entries survive.
#[test]
fn persistence_round_trip() {
    let dir = tempdir();

    {
        let mut lib = BrushLibrary::load_from_disk(&dir);
        lib.add("persist".to_string(), vec![red(); 4], 2, 2, 15, &Context::default());
    }

    {
        let lib = BrushLibrary::load_from_disk(&dir);
        assert_eq!(lib.len(), 1);
        assert_eq!(lib.selected().unwrap().name, "persist");
        assert_eq!(lib.selected().unwrap().spacing, 15);
    }
}

/// Remove the last brush clears selection.
#[test]
fn remove_last_brush_clears_selection() {
    let dir = tempdir();
    let mut lib = BrushLibrary::load_from_disk(&dir);

    lib.add("only".to_string(), vec![red(); 4], 2, 2, 25, &Context::default());
    assert!(lib.selected().is_some());

    lib.remove(0);
    assert!(lib.selected().is_none());
}

/// Remove with out-of-bounds index is a no-op.
#[test]
fn remove_out_of_bounds_noop() {
    let dir = tempdir();
    let mut lib = BrushLibrary::load_from_disk(&dir);

    lib.add("survivor".to_string(), vec![red(); 4], 2, 2, 25, &Context::default());
    assert_eq!(lib.len(), 1);

    lib.remove(5);
    assert_eq!(lib.len(), 1);
}

/// Select with out-of-bounds index is a no-op.
#[test]
fn select_out_of_bounds_noop() {
    let dir = tempdir();
    let mut lib = BrushLibrary::load_from_disk(&dir);

    lib.add("pickme".to_string(), vec![red(); 4], 2, 2, 25, &Context::default());
    assert_eq!(lib.selected_index(), Some(0));

    lib.select(42);
    assert_eq!(lib.selected_index(), Some(0));
}

/// Multiple brushes — remove middle, verify ordering preserved.
#[test]
fn remove_middle_preserves_order() {
    let dir = tempdir();
    let mut lib = BrushLibrary::load_from_disk(&dir);

    lib.add("first".to_string(), vec![red(); 4], 2, 2, 10, &Context::default());
    lib.add("second".to_string(), vec![red(); 4], 2, 2, 20, &Context::default());
    lib.add("third".to_string(), vec![red(); 4], 2, 2, 30, &Context::default());

    lib.remove(1);
    assert_eq!(lib.len(), 2);
    assert_eq!(lib.entries()[0].name, "first");
    assert_eq!(lib.entries()[1].name, "third");
}

/// `entries` on an empty library returns an empty slice.
#[test]
fn entries_empty_library() {
    let dir = tempdir();
    let lib = BrushLibrary::load_from_disk(&dir);
    assert!(lib.entries().is_empty());
}
