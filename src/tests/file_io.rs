use std::path::PathBuf;
use std::sync::mpsc;

use crate::canvas::Canvas;
use crate::document::Document;
use crate::file_io::{ DialogResult, FileIO, PendingFileAction, SaveKind, SaveResult };
use crate::undo_history::UndoHistory;

/// Create a `FileIO` with test channels and return it plus senders for
/// injecting dialog results and save results.
fn test_file_io() -> (FileIO, mpsc::Sender<DialogResult>, mpsc::Sender<SaveResult>) {
    let (dialog_tx, dialog_rx) = mpsc::channel();
    let (save_tx, save_rx) = mpsc::channel();
    let dialog_tx_clone = dialog_tx.clone();
    let save_tx_clone = save_tx.clone();
    let file_io = FileIO::new(dialog_tx, dialog_rx, save_tx, save_rx, PathBuf::from("/tmp"));
    (file_io, dialog_tx_clone, save_tx_clone)
}

// --- poll_save_results ---

#[test]
fn poll_save_results_autosave_clears_dirty() {
    let (file_io, _, save_tx) = test_file_io();
    let mut doc = Document::new(Canvas::new(10, 10));
    doc.dirty_since_last_autosave = true;
    let mut errors = Vec::new();

    save_tx.send(SaveResult::Autosave).unwrap();
    file_io.poll_save_results(&mut doc, &mut errors);

    assert!(!doc.dirty_since_last_autosave);
    assert!(errors.is_empty());
}

#[test]
fn poll_save_results_manual_save_sets_path() {
    let (file_io, _, save_tx) = test_file_io();
    let mut doc = Document::new(Canvas::new(10, 10));
    let mut errors = Vec::new();
    let path = PathBuf::from("/tmp/test.splattercanvas");

    save_tx.send(SaveResult::ManualSave(path.clone())).unwrap();
    file_io.poll_save_results(&mut doc, &mut errors);

    assert_eq!(doc.savefile_path, path.display().to_string());
    assert!(doc.canvas.render_next_frame);
    assert!(errors.is_empty());
}

#[test]
fn poll_save_results_failed_appends_error() {
    let (file_io, _, save_tx) = test_file_io();
    let mut doc = Document::new(Canvas::new(10, 10));
    let mut errors = Vec::new();

    save_tx.send(SaveResult::Failed("disk full".into())).unwrap();
    file_io.poll_save_results(&mut doc, &mut errors);

    assert!(errors.iter().any(|e| e.contains("disk full")));
}

#[test]
fn poll_save_results_no_messages_is_noop() {
    let (file_io, _, _) = test_file_io();
    let mut doc = Document::new(Canvas::new(10, 10));
    let mut errors = Vec::new();

    file_io.poll_save_results(&mut doc, &mut errors);

    assert!(errors.is_empty());
}

// --- poll_dialog_results ---

#[test]
fn poll_dialog_results_save_triggers_async_save() {
    let (dialog_tx, dialog_rx) = mpsc::channel();
    let (save_tx, save_rx) = mpsc::channel();
    let mut file_io = FileIO::new(dialog_tx.clone(), dialog_rx, save_tx, save_rx, PathBuf::from("/tmp"));
    file_io.pending_file_action = Some(PendingFileAction::Save);
    let mut doc = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    dialog_tx.send(DialogResult::Picked(PathBuf::from("/tmp/test.splattercanvas"))).unwrap();
    file_io.poll_dialog_results(&mut doc, &mut undo, &mut errors);

    // pending_file_action should have been consumed
    assert!(file_io.pending_file_action.is_none());
    assert!(errors.is_empty());
}

#[test]
fn poll_dialog_results_mismatched_pending_skips() {
    let (mut file_io, dialog_tx, _) = test_file_io();
    file_io.pending_file_action = Some(PendingFileAction::Load);
    let mut doc = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    // Send a dialog result without setting matching pending action
    // pending_file_action was set to Load, but we'll set it to None after taking
    dialog_tx.send(DialogResult::Picked(PathBuf::from("/tmp/test.splattercanvas"))).unwrap();
    file_io.poll_dialog_results(&mut doc, &mut undo, &mut errors);

    // No error, message consumed but skipped because pending didn't match
    assert!(errors.is_empty());
}

// --- save_to_current_path ---

#[test]
fn save_to_current_path_empty_path_noop() {
    let (file_io, _, _) = test_file_io();
    let doc = Document::new(Canvas::new(10, 10));
    // Should not panic or spawn thread
    file_io.save_to_current_path(&doc);
}

// --- trigger_async_save ---

#[test]
fn trigger_async_save_writes_file() {
    let (file_io, _, _) = test_file_io();
    let canvas = Canvas::new(10, 10);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.splattercanvas");

    file_io.trigger_async_save(&Document::new(canvas), SaveKind::ManualSave(path.clone()));

    // Wait a bit for the async thread to finish
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(path.exists(), "file should exist at {path:?}");
}
