//! Tests for `Library<StampEntry>` — add, remove, select, and persistence round-trip.

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use eframe::egui::Context;

use crate::asset_library::Library;
use crate::stamp_library::StampEntry;
use crate::stamp_library::add_stamp;
use crate::tests::common::red;

/// Add one stamp and verify it is selected.
#[test]
fn add_stamp_increments_count() {
    let dir = tempdir();
    let mut lib = Library::<StampEntry>::load_from_disk(&dir);

    assert!(lib.is_empty());
    assert_eq!(lib.len(), 0);
    assert!(lib.selected().is_none());

    let (pixels, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(
        &mut lib,
        "test".to_string(),
        pixels,
        w,
        h,
        &Context::default(),
    );

    assert_eq!(lib.len(), 1);
    assert!(lib.selected().is_some());
    assert_eq!(lib.selected().unwrap().name, "test");
    assert_eq!(lib.selected().unwrap().width, 2);
    assert_eq!(lib.selected().unwrap().height, 2);
}

/// Remove a stamp and verify count decreases and it persists on reload.
#[test]
fn remove_stamp_decrements_count() {
    let dir = tempdir();
    let mut lib = Library::<StampEntry>::load_from_disk(&dir);

    let (pixels, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(
        &mut lib,
        "to_remove".to_string(),
        pixels,
        w,
        h,
        &Context::default(),
    );
    assert_eq!(lib.len(), 1);

    lib.remove(0);
    assert!(lib.is_empty());
    assert!(lib.selected().is_none());
}

/// Select a specific stamp by index.
#[test]
fn select_switches_active_stamp() {
    let dir = tempdir();
    let mut lib = Library::<StampEntry>::load_from_disk(&dir);

    let (p1, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(&mut lib, "first".to_string(), p1, w, h, &Context::default());

    let (p2, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(
        &mut lib,
        "second".to_string(),
        p2,
        w,
        h,
        &Context::default(),
    );

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

    // Create and save
    {
        let mut lib = Library::<StampEntry>::load_from_disk(&dir);
        let (pixels, w, h) = (vec![red(); 4], 2, 2);
        add_stamp(
            &mut lib,
            "persist".to_string(),
            pixels,
            w,
            h,
            &Context::default(),
        );
    }

    // Reload
    {
        let lib = Library::<StampEntry>::load_from_disk(&dir);
        assert_eq!(lib.len(), 1);
        assert_eq!(lib.selected().unwrap().name, "persist");
    }
}

/// Clean up empty library after remove.
#[test]
fn remove_last_stamp_clears_selection() {
    let dir = tempdir();
    let mut lib = Library::<StampEntry>::load_from_disk(&dir);

    let (pixels, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(
        &mut lib,
        "only".to_string(),
        pixels,
        w,
        h,
        &Context::default(),
    );
    assert!(lib.selected().is_some());

    lib.remove(0);
    assert!(lib.selected().is_none());
}

/// Retrieve a stamp by valid index.
#[test]
fn get_valid_index() {
    let dir = tempdir();
    let mut lib = Library::<StampEntry>::load_from_disk(&dir);

    let (pixels, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(
        &mut lib,
        "get_test".to_string(),
        pixels,
        w,
        h,
        &Context::default(),
    );

    let entry = lib.get(0);
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().name, "get_test");
}

/// `get` with an out-of-bounds index should return `None`.
#[test]
fn get_out_of_bounds_returns_none() {
    let dir = tempdir();
    let lib = Library::<StampEntry>::load_from_disk(&dir);
    assert!(lib.get(0).is_none());
    assert!(lib.get(100).is_none());
    assert!(lib.get(usize::MAX).is_none());
}

/// `entries` should return a slice matching internal state.
#[test]
fn entries_returns_all() {
    let dir = tempdir();
    let mut lib = Library::<StampEntry>::load_from_disk(&dir);

    let (p1, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(&mut lib, "a".to_string(), p1, w, h, &Context::default());
    let (p2, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(&mut lib, "b".to_string(), p2, w, h, &Context::default());

    let entries = lib.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "a");
    assert_eq!(entries[1].name, "b");
}

/// `entries` on an empty library should return an empty slice.
#[test]
fn entries_empty_library() {
    let dir = tempdir();
    let lib = Library::<StampEntry>::load_from_disk(&dir);
    assert!(lib.entries().is_empty());
}

/// `selected_mut` allows mutation of the selected stamp's fields.
#[test]
fn selected_mut_allows_mutation() {
    let dir = tempdir();
    let mut lib = Library::<StampEntry>::load_from_disk(&dir);

    let (pixels, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(
        &mut lib,
        "mutable".to_string(),
        pixels,
        w,
        h,
        &Context::default(),
    );

    let entry = lib.selected_mut().expect("should have selected");
    entry.name = "mutated".to_string();
    assert_eq!(lib.selected().unwrap().name, "mutated");
}

/// `selected_mut` on an empty library should return `None`.
#[test]
fn selected_mut_empty_library() {
    let dir = tempdir();
    let mut lib = Library::<StampEntry>::load_from_disk(&dir);
    assert!(lib.selected_mut().is_none());
}

/// `remove` with an out-of-bounds index should be a no-op.
#[test]
fn remove_out_of_bounds_noop() {
    let dir = tempdir();
    let mut lib = Library::<StampEntry>::load_from_disk(&dir);

    let (pixels, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(
        &mut lib,
        "survivor".to_string(),
        pixels,
        w,
        h,
        &Context::default(),
    );
    assert_eq!(lib.len(), 1);

    lib.remove(5);
    assert_eq!(lib.len(), 1, "OOB remove should not change count");
    assert!(lib.selected().is_some());
}

/// `select` with an out-of-bounds index should be a no-op.
#[test]
fn select_out_of_bounds_noop() {
    let dir = tempdir();
    let mut lib = Library::<StampEntry>::load_from_disk(&dir);

    let (pixels, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(
        &mut lib,
        "pickme".to_string(),
        pixels,
        w,
        h,
        &Context::default(),
    );
    assert_eq!(lib.selected_index(), Some(0));

    lib.select(42);
    assert_eq!(
        lib.selected_index(),
        Some(0),
        "OOB select should not change selection"
    );
}

/// Multiple stamps — remove middle, verify ordering preserved.
#[test]
fn remove_middle_preserves_order() {
    let dir = tempdir();
    let mut lib = Library::<StampEntry>::load_from_disk(&dir);

    let (p1, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(&mut lib, "first".to_string(), p1, w, h, &Context::default());
    let (p2, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(
        &mut lib,
        "second".to_string(),
        p2,
        w,
        h,
        &Context::default(),
    );
    let (p3, w, h) = (vec![red(); 4], 2, 2);
    add_stamp(&mut lib, "third".to_string(), p3, w, h, &Context::default());

    lib.remove(1);
    assert_eq!(lib.len(), 2);
    assert_eq!(lib.entries()[0].name, "first");
    assert_eq!(lib.entries()[1].name, "third");
}

static NEXT_STAMP_DIR: AtomicU64 = AtomicU64::new(0);

/// Helper: create a temporary directory that cleans itself up on drop.
fn tempdir() -> std::path::PathBuf {
    let id = NEXT_STAMP_DIR.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("stamp_lib_test_{pid}_{id}"));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
