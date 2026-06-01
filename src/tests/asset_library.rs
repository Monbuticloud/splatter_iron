//! Tests for the generic [`Library<T>`] asset storage and [`AssetEntry`] trait.
//!
//! Uses a minimal test entry type (`TestEntry`) to verify the generic
//! machinery independently of brush/stamp specialisations.

use std::sync::atomic::{AtomicU64, Ordering};

use eframe::egui::Color32;
use eframe::egui::TextureHandle;

use crate::asset_library::AssetEntry;
use crate::asset_library::Library;

/// Minimal asset entry for testing the generic library.
struct TestEntry {
    name: String,
    filename: String,
    pixels: Vec<Color32>,
    width: u32,
    height: u32,
    texture_handle: Option<TextureHandle>,
}

impl std::fmt::Debug for TestEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestEntry")
            .field("name", &self.name)
            .field("filename", &self.filename)
            .field("pixels.len", &self.pixels.len())
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl AssetEntry for TestEntry {
    fn name(&self) -> &str {
        &self.name
    }
    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
    fn filename(&self) -> &str {
        &self.filename
    }
    fn filename_mut(&mut self) -> &mut String {
        &mut self.filename
    }
    fn pixels(&self) -> &[Color32] {
        &self.pixels
    }
    fn pixels_mut(&mut self) -> &mut Vec<Color32> {
        &mut self.pixels
    }
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn texture_handle(&self) -> &Option<TextureHandle> {
        &self.texture_handle
    }
    fn texture_handle_mut(&mut self) -> &mut Option<TextureHandle> {
        &mut self.texture_handle
    }

    fn dir_name() -> &'static str {
        "test_assets"
    }
    fn json_field_name() -> &'static str {
        "test_assets"
    }

    fn extra_index_fields(&self) -> Vec<(&'static str, serde_json::Value)> {
        Vec::new()
    }

    fn from_parts(
        name: String,
        filename: String,
        pixels: Vec<Color32>,
        w: u32,
        h: u32,
        _extra: &serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        Self {
            name,
            filename,
            pixels,
            width: w,
            height: h,
            texture_handle: None,
        }
    }
}

static NEXT_ASSET_DIR: AtomicU64 = AtomicU64::new(0);

fn tempdir() -> std::path::PathBuf {
    let id = NEXT_ASSET_DIR.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("asset_lib_test_{id}"));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn test_entry(name: &str) -> TestEntry {
    TestEntry {
        name: name.to_string(),
        filename: String::new(),
        pixels: vec![Color32::from_rgba_premultiplied(255, 0, 0, 255); 4],
        width: 2,
        height: 2,
        texture_handle: None,
    }
}

/// Loading from a non-existent directory creates it and returns an empty library.
#[test]
fn load_from_disk_creates_dir() {
    let dir = tempdir();
    let lib = Library::<TestEntry>::load_from_disk(&dir);
    assert!(lib.is_empty());
    assert!(lib.selected().is_none());
    assert!(dir.join("test_assets").exists());
}

/// Adding an entry increments the count and selects it.
#[test]
fn add_entry_increments_count() {
    let dir = tempdir();
    let mut lib = Library::<TestEntry>::load_from_disk(&dir);
    lib.add_entry(test_entry("alpha"), &eframe::egui::Context::default());
    assert_eq!(lib.len(), 1);
    assert_eq!(lib.selected().unwrap().name(), "alpha");
}

/// Adding multiple entries preserves insertion order.
#[test]
fn add_multiple_preserves_order() {
    let dir = tempdir();
    let mut lib = Library::<TestEntry>::load_from_disk(&dir);
    lib.add_entry(test_entry("first"), &eframe::egui::Context::default());
    lib.add_entry(test_entry("second"), &eframe::egui::Context::default());
    lib.add_entry(test_entry("third"), &eframe::egui::Context::default());
    assert_eq!(lib.len(), 3);
    assert_eq!(lib.get(0).unwrap().name(), "first");
    assert_eq!(lib.get(1).unwrap().name(), "second");
    assert_eq!(lib.get(2).unwrap().name(), "third");
}

/// Remove the only entry clears selection.
#[test]
fn remove_last_clears_selection() {
    let dir = tempdir();
    let mut lib = Library::<TestEntry>::load_from_disk(&dir);
    lib.add_entry(test_entry("only"), &eframe::egui::Context::default());
    assert!(lib.selected().is_some());
    lib.remove(0);
    assert!(lib.selected().is_none());
    assert!(lib.is_empty());
}

/// Remove the middle entry preserves ordering of the rest.
#[test]
fn remove_middle_preserves_order() {
    let dir = tempdir();
    let mut lib = Library::<TestEntry>::load_from_disk(&dir);
    lib.add_entry(test_entry("a"), &eframe::egui::Context::default());
    lib.add_entry(test_entry("b"), &eframe::egui::Context::default());
    lib.add_entry(test_entry("c"), &eframe::egui::Context::default());
    lib.remove(1);
    assert_eq!(lib.len(), 2);
    assert_eq!(lib.get(0).unwrap().name(), "a");
    assert_eq!(lib.get(1).unwrap().name(), "c");
}

/// Select switches which entry is active.
#[test]
fn select_switches_active() {
    let dir = tempdir();
    let mut lib = Library::<TestEntry>::load_from_disk(&dir);
    lib.add_entry(test_entry("x"), &eframe::egui::Context::default());
    lib.add_entry(test_entry("y"), &eframe::egui::Context::default());
    lib.select(0);
    assert_eq!(lib.selected().unwrap().name(), "x");
    lib.select(1);
    assert_eq!(lib.selected().unwrap().name(), "y");
}

/// Select with out-of-bounds index is a no-op.
#[test]
fn select_out_of_bounds_noop() {
    let dir = tempdir();
    let mut lib = Library::<TestEntry>::load_from_disk(&dir);
    lib.add_entry(test_entry("only"), &eframe::egui::Context::default());
    lib.select(42);
    assert_eq!(lib.selected_index(), Some(0));
}

/// `get` with valid index returns the entry.
#[test]
fn get_valid_index() {
    let dir = tempdir();
    let mut lib = Library::<TestEntry>::load_from_disk(&dir);
    lib.add_entry(test_entry("found"), &eframe::egui::Context::default());
    assert!(lib.get(0).is_some());
    assert_eq!(lib.get(0).unwrap().name(), "found");
}

/// `get` with out-of-bounds index returns None.
#[test]
fn get_out_of_bounds_none() {
    let dir = tempdir();
    let lib = Library::<TestEntry>::load_from_disk(&dir);
    assert!(lib.get(0).is_none());
    assert!(lib.get(100).is_none());
}

/// `entries` on empty library returns empty slice.
#[test]
fn entries_empty() {
    let dir = tempdir();
    let lib = Library::<TestEntry>::load_from_disk(&dir);
    assert!(lib.entries().is_empty());
}

/// `entries` returns all added entries.
#[test]
fn entries_returns_all() {
    let dir = tempdir();
    let mut lib = Library::<TestEntry>::load_from_disk(&dir);
    lib.add_entry(test_entry("a"), &eframe::egui::Context::default());
    lib.add_entry(test_entry("b"), &eframe::egui::Context::default());
    assert_eq!(lib.entries().len(), 2);
}

/// Persistence round-trip: entries survive a reload.
#[test]
fn persistence_round_trip() {
    let dir = tempdir();
    {
        let mut lib = Library::<TestEntry>::load_from_disk(&dir);
        lib.add_entry(test_entry("persist"), &eframe::egui::Context::default());
    }
    {
        let lib = Library::<TestEntry>::load_from_disk(&dir);
        assert_eq!(lib.len(), 1);
        assert_eq!(lib.selected().unwrap().name(), "persist");
    }
}

/// `selected_mut` allows mutation of the selected entry.
#[test]
fn selected_mut_allows_mutation() {
    let dir = tempdir();
    let mut lib = Library::<TestEntry>::load_from_disk(&dir);
    lib.add_entry(test_entry("old"), &eframe::egui::Context::default());
    let entry = lib.selected_mut().expect("should be selected");
    *entry.name_mut() = "new".to_string();
    assert_eq!(lib.selected().unwrap().name(), "new");
}

/// `selected_mut` on empty library returns None.
#[test]
fn selected_mut_empty_none() {
    let dir = tempdir();
    let mut lib = Library::<TestEntry>::load_from_disk(&dir);
    assert!(lib.selected_mut().is_none());
}

/// Remove with out-of-bounds index is a no-op.
#[test]
fn remove_out_of_bounds_noop() {
    let dir = tempdir();
    let mut lib = Library::<TestEntry>::load_from_disk(&dir);
    lib.add_entry(test_entry("survivor"), &eframe::egui::Context::default());
    lib.remove(5);
    assert_eq!(lib.len(), 1);
}
