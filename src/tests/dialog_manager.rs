//! Tests for [`DialogManager`] — dialog queueing, `DispatchedAction` dispatching,
//! stamp/brush data storage.

use std::path::PathBuf;
use std::sync::mpsc;

use eframe::egui::Color32;

use crate::file_io::DialogManager;
use crate::file_io::DialogResult;
use crate::file_io::DispatchedAction;
use crate::file_io::PendingFileAction;

/// Create a [`DialogManager`] with test channels and return the sender for
/// injecting dialog results.
fn create_dialog_manager() -> (DialogManager, mpsc::Sender<DialogResult>) {
    let (dialog_sender, dialog_receiver) = mpsc::channel();
    let sender_clone = dialog_sender.clone();
    let dm = DialogManager::new(dialog_sender, dialog_receiver);
    (dm, sender_clone)
}

// --- poll_dialog_results — Save ---

#[test]
fn poll_dialog_results_save_returns_dispatched_action() {
    let (mut dm, dialog_sender) = create_dialog_manager();
    dm.pending_file_action = Some(PendingFileAction::Save);
    let mut errors = Vec::new();

    dialog_sender
        .send(DialogResult::Picked(PathBuf::from(
            "/tmp/test.splattercanvas",
        )))
        .unwrap();
    let actions = dm.poll_dialog_results(&mut errors);

    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], DispatchedAction::Save(_)));
    assert!(dm.pending_file_action.is_none());
    assert!(errors.is_empty());
}

// --- poll_dialog_results — Load ---

#[test]
fn poll_dialog_results_load_returns_dispatched_action() {
    let (mut dm, dialog_sender) = create_dialog_manager();
    dm.pending_file_action = Some(PendingFileAction::Load);
    let mut errors = Vec::new();

    dialog_sender
        .send(DialogResult::Picked(PathBuf::from(
            "/tmp/test.splattercanvas",
        )))
        .unwrap();
    let actions = dm.poll_dialog_results(&mut errors);

    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], DispatchedAction::Load(_)));
    assert!(errors.is_empty());
}

// --- poll_dialog_results — mismatched pending ---

#[test]
fn poll_dialog_results_mismatched_pending_skips() {
    let (mut dm, dialog_sender) = create_dialog_manager();
    dm.pending_file_action = Some(PendingFileAction::Load);
    let mut errors = Vec::new();

    dialog_sender
        .send(DialogResult::Picked(PathBuf::from(
            "/tmp/test.splattercanvas",
        )))
        .unwrap();
    // pending_file_action was Load — after poll it gets consumed.
    let _ = dm.poll_dialog_results(&mut errors);

    assert!(errors.is_empty());
}

// --- poll_dialog_results — StampPixels ---

#[test]
fn poll_dialog_results_stamp_pixels_sets_loaded() {
    let (mut dm, dialog_sender) = create_dialog_manager();
    let mut errors = Vec::new();

    let pixels = vec![Color32::RED; 4];
    dialog_sender
        .send(DialogResult::StampPixels(
            pixels.clone(),
            2,
            2,
            "stamp_name".to_string(),
        ))
        .unwrap();
    let _ = dm.poll_dialog_results(&mut errors);

    assert!(dm.loaded_stamp_data.is_some());
    let (loaded_pixels, w, h, name) = dm.loaded_stamp_data.take().unwrap();
    assert_eq!(loaded_pixels, pixels);
    assert_eq!(w, 2);
    assert_eq!(h, 2);
    assert_eq!(name, "stamp_name");
    assert!(errors.is_empty());
}

// --- poll_dialog_results — Error ---

#[test]
fn poll_dialog_results_error_appends() {
    let (mut dm, dialog_sender) = create_dialog_manager();
    let mut errors = Vec::new();

    dialog_sender
        .send(DialogResult::Error("test error message".to_string()))
        .unwrap();
    let _ = dm.poll_dialog_results(&mut errors);

    assert!(errors.iter().any(|e| e.contains("test error message")));
    assert!(dm.pending_file_action.is_none());
}

// --- poll_dialog_results — Cancelled ---

#[test]
fn poll_dialog_results_cancelled_clears_pending() {
    let (mut dm, dialog_sender) = create_dialog_manager();
    dm.pending_file_action = Some(PendingFileAction::Save);
    let mut errors = Vec::new();

    dialog_sender.send(DialogResult::Cancelled).unwrap();
    let _ = dm.poll_dialog_results(&mut errors);

    assert!(dm.pending_file_action.is_none());
    assert!(errors.is_empty());
}

// --- queue_file_action — Save ---

#[test]
fn queue_file_action_save_sets_pending() {
    let (mut dm, _) = create_dialog_manager();
    assert!(dm.pending_file_action.is_none());
    dm.queue_file_action(PendingFileAction::Save);
    assert!(dm.pending_file_action.is_some());
    let _ = dm.pending_file_action.take();
}

// --- queue_file_action — LoadStamp ---

#[test]
fn queue_file_action_load_stamp_sets_pending() {
    let (mut dm, _) = create_dialog_manager();
    dm.queue_file_action(PendingFileAction::LoadStamp);
    assert!(dm.pending_file_action.is_some());
    let _ = dm.pending_file_action.take();
}

// --- queue_file_action — Export ---

#[test]
fn queue_file_action_export_sets_pending() {
    let (mut dm, _) = create_dialog_manager();
    dm.queue_file_action(PendingFileAction::Export(0));
    assert!(dm.pending_file_action.is_some());
    let _ = dm.pending_file_action.take();
}

// --- queue_file_action — ExportArchive ---

#[test]
fn queue_file_action_export_archive_sets_pending() {
    let (mut dm, _) = create_dialog_manager();
    dm.queue_file_action(PendingFileAction::ExportArchive);
    assert!(dm.pending_file_action.is_some());
    let _ = dm.pending_file_action.take();
}

// --- queue_file_action — ImportArchive ---

#[test]
fn queue_file_action_import_archive_sets_pending() {
    let (mut dm, _) = create_dialog_manager();
    dm.queue_file_action(PendingFileAction::ImportArchive);
    assert!(dm.pending_file_action.is_some());
    let _ = dm.pending_file_action.take();
}

// --- poll_dialog_results — ExportArchive dispatches action ---

#[test]
fn poll_dialog_results_export_archive_returns_action() {
    let (mut dm, dialog_sender) = create_dialog_manager();
    dm.pending_file_action = Some(PendingFileAction::ExportArchive);
    let mut errors = Vec::new();

    let dir = tempfile::tempdir().expect("temp dir");
    let archive_path = dir.path().join("test.splatterarchive");
    dialog_sender
        .send(DialogResult::Picked(archive_path))
        .unwrap();
    let actions = dm.poll_dialog_results(&mut errors);

    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], DispatchedAction::ExportArchive(_)));
    assert!(dm.pending_file_action.is_none());
    assert!(errors.is_empty());
}

// --- Save path appends extension ---

#[test]
fn poll_dialog_results_save_appends_extension() {
    let (mut dm, dialog_sender) = create_dialog_manager();
    dm.pending_file_action = Some(PendingFileAction::Save);
    let mut errors = Vec::new();

    dialog_sender
        .send(DialogResult::Picked(PathBuf::from("/tmp/test")))
        .unwrap();
    let actions = dm.poll_dialog_results(&mut errors);

    assert_eq!(actions.len(), 1);
    if let DispatchedAction::Save(path) = &actions[0] {
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".splattercanvas"),
            "expected .splattercanvas extension, got {path_str}"
        );
    } else {
        panic!("expected Save action");
    }
    assert!(errors.is_empty());
}
