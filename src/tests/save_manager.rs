//! Tests for [`SaveManager`] — async save orchestration, result polling.

use std::path::PathBuf;
use std::sync::mpsc;

use crate::canvas::Canvas;
use crate::document::Document;
use crate::file_io::SaveKind;
use crate::file_io::SaveManager;
use crate::file_io::SaveResult;

/// Create a [`SaveManager`] with test channels and return it plus a sender
/// for injecting save results.
fn create_save_manager() -> (SaveManager, mpsc::Sender<SaveResult>) {
    let (save_sender, save_receiver) = mpsc::channel();
    let sender_clone = save_sender.clone();
    let sm = SaveManager::new(save_sender, save_receiver, PathBuf::from("/tmp"));
    (sm, sender_clone)
}

// --- poll_save_results ---

#[test]
fn poll_save_results_autosave_clears_dirty() {
    let (mut sm, save_sender) = create_save_manager();
    let mut document = Document::new(Canvas::new(10, 10));
    document.dirty_since_last_autosave = true;
    let mut errors = Vec::new();

    save_sender.send(SaveResult::Autosave).unwrap();
    sm.poll_save_results(&mut document, &mut errors);

    assert!(!document.dirty_since_last_autosave);
    assert!(errors.is_empty());
}

#[test]
fn poll_save_results_manual_save_sets_path() {
    let (mut sm, save_sender) = create_save_manager();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut errors = Vec::new();
    let path = PathBuf::from("/tmp/test.splattercanvas");

    save_sender
        .send(SaveResult::ManualSave(path.clone()))
        .unwrap();
    sm.poll_save_results(&mut document, &mut errors);

    assert_eq!(document.savefile_path, path.display().to_string());
    assert!(document.canvas.dirty_rect.needs_reblend());
    assert!(errors.is_empty());
}

#[test]
fn poll_save_results_failed_appends_error() {
    let (mut sm, save_sender) = create_save_manager();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut errors = Vec::new();

    save_sender
        .send(SaveResult::Failed("disk full".into()))
        .unwrap();
    sm.poll_save_results(&mut document, &mut errors);

    assert!(errors.iter().any(|e| e.contains("disk full")));
}

#[test]
fn poll_save_results_no_messages_is_noop() {
    let (mut sm, _) = create_save_manager();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut errors = Vec::new();

    sm.poll_save_results(&mut document, &mut errors);
    assert!(errors.is_empty());
}

#[test]
fn poll_save_results_manual_save_empty_path() {
    let (mut sm, save_sender) = create_save_manager();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut errors = Vec::new();

    save_sender
        .send(SaveResult::ManualSave(PathBuf::new()))
        .unwrap();
    sm.poll_save_results(&mut document, &mut errors);
    assert!(errors.is_empty());
}

// --- save_to_current_path ---

#[test]
fn save_to_current_path_empty_path_noop() {
    let (mut sm, _) = create_save_manager();
    let mut document = Document::new(Canvas::new(10, 10));
    sm.save_to_current_path(&mut document);
}

#[test]
fn save_to_current_path_non_empty_triggers_save() {
    let (save_sender, save_receiver) = mpsc::channel();
    let mut sm = SaveManager::new(
        save_sender,
        save_receiver,
        PathBuf::from("/tmp"),
    );
    let mut document = Document::new(Canvas::new(1, 1));
    document.savefile_path = "/tmp/test_save_non_empty.splattercanvas".to_string();
    sm.save_to_current_path(&mut document);
    std::thread::sleep(std::time::Duration::from_millis(200));
    // The async save thread was spawned without panic.
}

// --- trigger_async_save ---

#[test]
fn trigger_async_save_writes_file() {
    let (mut sm, _) = create_save_manager();
    let canvas = Canvas::new(10, 10);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.splattercanvas");

    let mut doc = Document::new(canvas);
    sm.trigger_async_save(&mut doc, SaveKind::ManualSave(path.clone()));

    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(path.exists(), "file should exist at {path:?}");
}

// --- autosave_directory ---

#[test]
fn autosave_directory_path() {
    let (sm, _) = create_save_manager();
    let expected = PathBuf::from("/tmp").join("autosaves");
    assert_eq!(sm.autosave_directory(), expected);
}
