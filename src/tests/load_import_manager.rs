//! Tests for [`LoadImportManager`] — async load/import orchestration, result polling.

use crate::canvas::Canvas;
use crate::document::Document;
use crate::file_io::LoadImportManager;
use crate::file_io::LoadImportResult;
use crate::undo_history::UndoHistory;

/// Create a [`LoadImportManager`] with an internal channel pair.
fn create_load_import_manager() -> LoadImportManager {
    LoadImportManager::new()
}

// --- poll_load_import_results ---

#[test]
fn poll_load_import_results_loaded_replaces_canvas() {
    let mut lim = create_load_import_manager();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    let source = Canvas::new(3, 4);
    lim.load_import_sender
        .send(LoadImportResult::Loaded(
            source,
            "/tmp/test.splattercanvas".into(),
        ))
        .unwrap();
    lim.poll_load_import_results(&mut document, &mut undo, &mut errors);

    assert!(errors.is_empty(), "errors: {errors:?}");
    assert_eq!(document.canvas.width, 3);
    assert_eq!(document.canvas.height, 4);
    assert!(document.canvas.dirty_rect.needs_reblend());
    assert!(!lim.load_in_flight);
}

#[test]
fn poll_load_import_results_imported_replaces_canvas() {
    use eframe::egui::Color32;

    use crate::canvas::Layer;

    let mut lim = create_load_import_manager();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    let layers = vec![Layer {
        pixels: vec![Color32::TRANSPARENT; 6],
        ..Default::default()
    }];
    lim.load_import_sender
        .send(LoadImportResult::Imported(layers, 2, 3))
        .unwrap();
    lim.poll_load_import_results(&mut document, &mut undo, &mut errors);

    assert!(errors.is_empty(), "errors: {errors:?}");
    assert_eq!(document.canvas.width, 2);
    assert_eq!(document.canvas.height, 3);
    assert!(document.canvas.dirty_rect.needs_reblend());
    assert!(!lim.import_in_flight);
}

#[test]
fn poll_load_import_results_archive_imported_replaces_canvas() {
    let mut lim = create_load_import_manager();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    let imported = Canvas::new(5, 7);
    lim.load_import_sender
        .send(LoadImportResult::ArchiveImported(imported))
        .unwrap();
    lim.poll_load_import_results(&mut document, &mut undo, &mut errors);

    assert!(errors.is_empty(), "errors: {errors:?}");
    assert_eq!(document.canvas.width, 5);
    assert_eq!(document.canvas.height, 7);
    assert!(document.canvas.dirty_rect.needs_reblend());
}

#[test]
fn poll_load_import_results_failed_appends_error() {
    let mut lim = create_load_import_manager();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    lim.load_import_sender
        .send(LoadImportResult::Failed("import failed".to_string()))
        .unwrap();
    lim.poll_load_import_results(&mut document, &mut undo, &mut errors);

    assert!(errors.iter().any(|e| e.contains("import failed")));
    assert!(!lim.load_in_flight);
    assert!(!lim.import_in_flight);
}

#[test]
fn poll_load_import_results_no_messages_is_noop() {
    let mut lim = create_load_import_manager();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    lim.poll_load_import_results(&mut document, &mut undo, &mut errors);
    assert!(errors.is_empty());
}

// --- trigger_async_load ---

#[test]
fn trigger_async_load_nonexistent_file_fails() {
    let mut lim = create_load_import_manager();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("does_not_exist.splattercanvas");
    lim.trigger_async_load(missing);

    for _ in 0..100 {
        lim.poll_load_import_results(&mut document, &mut undo, &mut errors);
        if !errors.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!errors.is_empty(), "expected load error for missing file");
}
