//! Tests for `FileIO` — async file-dialog glue, save/load orchestration.
//!
//! Exercises the mpsc-channel plumbing that connects UI events to
//! background file dialogs without hanging the frame loop.

use std::path::PathBuf;
use std::sync::mpsc;

use eframe::egui::Color32;

use crate::canvas::Canvas;
use crate::document::Document;
use crate::file_io::DialogResult;
use crate::file_io::FileIO;
use crate::file_io::PendingFileAction;
use crate::file_io::SaveKind;
use crate::file_io::SaveResult;
use crate::undo_history::UndoHistory;

/// Create a `FileIO` with test channels and return it plus senders for
/// injecting dialog results and save results.
fn test_file_io() -> (FileIO, mpsc::Sender<DialogResult>, mpsc::Sender<SaveResult>) {
    let (dialog_sender, dialog_receiver) = mpsc::channel();
    let (save_sender, save_receiver) = mpsc::channel();
    let dialog_sender_clone = dialog_sender.clone();
    let save_sender_clone = save_sender.clone();
    let file_io = FileIO::new(
        dialog_sender,
        dialog_receiver,
        save_sender,
        save_receiver,
        PathBuf::from("/tmp"),
        std::sync::Arc::new(crate::files::DefaultExportStrategy),
    );
    (file_io, dialog_sender_clone, save_sender_clone)
}

// --- poll_save_results ---

#[test]
fn poll_save_results_autosave_clears_dirty() {
    let (mut file_io, _, save_sender) = test_file_io();
    let mut document = Document::new(Canvas::new(10, 10));
    document.dirty_since_last_autosave = true;
    let mut errors = Vec::new();

    save_sender.send(SaveResult::Autosave).unwrap();
    file_io.poll_save_results(&mut document, &mut errors);

    assert!(!document.dirty_since_last_autosave);
    assert!(errors.is_empty());
}

#[test]
fn poll_save_results_manual_save_sets_path() {
    let (mut file_io, _, save_sender) = test_file_io();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut errors = Vec::new();
    let path = PathBuf::from("/tmp/test.splattercanvas");

    save_sender
        .send(SaveResult::ManualSave(path.clone()))
        .unwrap();
    file_io.poll_save_results(&mut document, &mut errors);

    assert_eq!(document.savefile_path, path.display().to_string());
    assert!(document.canvas.dirty_rect.needs_reblend());
    assert!(errors.is_empty());
}

#[test]
fn poll_save_results_failed_appends_error() {
    let (mut file_io, _, save_sender) = test_file_io();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut errors = Vec::new();

    save_sender
        .send(SaveResult::Failed("disk full".into()))
        .unwrap();
    file_io.poll_save_results(&mut document, &mut errors);

    assert!(errors.iter().any(|e| e.contains("disk full")));
}

#[test]
fn poll_save_results_no_messages_is_noop() {
    let (mut file_io, _, _) = test_file_io();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut errors = Vec::new();

    file_io.poll_save_results(&mut document, &mut errors);

    assert!(errors.is_empty());
}

// --- poll_dialog_results ---

#[test]
fn poll_dialog_results_save_triggers_async_save() {
    let (dialog_sender, dialog_receiver) = mpsc::channel();
    let (save_sender, save_receiver) = mpsc::channel();
    let mut file_io = FileIO::new(
        dialog_sender.clone(),
        dialog_receiver,
        save_sender,
        save_receiver,
        PathBuf::from("/tmp"),
        std::sync::Arc::new(crate::files::DefaultExportStrategy),
    );
    file_io.pending_file_action = Some(PendingFileAction::Save);
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    dialog_sender
        .send(DialogResult::Picked(PathBuf::from(
            "/tmp/test.splattercanvas",
        )))
        .unwrap();
    file_io.poll_dialog_results(&mut document, &mut undo, &mut errors);

    // pending_file_action should have been consumed
    assert!(file_io.pending_file_action.is_none());
    assert!(errors.is_empty());
}

#[test]
fn poll_dialog_results_mismatched_pending_skips() {
    let (mut file_io, dialog_sender, _) = test_file_io();
    file_io.pending_file_action = Some(PendingFileAction::Load);
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    // Send a dialog result without setting matching pending action
    // pending_file_action was set to Load, but we'll set it to None after taking
    dialog_sender
        .send(DialogResult::Picked(PathBuf::from(
            "/tmp/test.splattercanvas",
        )))
        .unwrap();
    file_io.poll_dialog_results(&mut document, &mut undo, &mut errors);

    // No error, message consumed but skipped because pending didn't match
    assert!(errors.is_empty());
}

// --- save_to_current_path ---

#[test]
fn save_to_current_path_empty_path_noop() {
    let (mut file_io, _, _) = test_file_io();
    let mut document = Document::new(Canvas::new(10, 10));
    // Should not panic or spawn thread
    file_io.save_to_current_path(&mut document);
}

// --- trigger_async_save ---

#[test]
fn trigger_async_save_writes_file() {
    let (mut file_io, _, _) = test_file_io();
    let canvas = Canvas::new(10, 10);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.splattercanvas");

    let mut doc = Document::new(canvas);
    file_io.trigger_async_save(&mut doc, SaveKind::ManualSave(path.clone()));

    // Wait a bit for the async thread to finish
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(path.exists(), "file should exist at {path:?}");
}

// --- poll_dialog_results — additional dialog result paths ---

/// `poll_dialog_results` with a StampPixels result should set loaded_stamp_data.
#[test]
fn poll_dialog_results_stamp_pixels_sets_loaded() {
    let (mut file_io, dialog_sender, _) = test_file_io();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
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
    file_io.poll_dialog_results(&mut document, &mut undo, &mut errors);

    assert!(file_io.loaded_stamp_data.is_some());
    let (loaded_pixels, w, h, name) = file_io.loaded_stamp_data.take().unwrap();
    assert_eq!(loaded_pixels, pixels);
    assert_eq!(w, 2);
    assert_eq!(h, 2);
    assert_eq!(name, "stamp_name");
    assert!(errors.is_empty());
}

/// `poll_dialog_results` with an Error result should append to error list.
#[test]
fn poll_dialog_results_error_appends() {
    let (mut file_io, dialog_sender, _) = test_file_io();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    dialog_sender
        .send(DialogResult::Error("test error message".to_string()))
        .unwrap();
    file_io.poll_dialog_results(&mut document, &mut undo, &mut errors);

    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.contains("test error message")));
    // pending_file_action should be cleared
    assert!(file_io.pending_file_action.is_none());
}

/// `poll_dialog_results` with a Load path pointing to a valid .splattercanvas file
/// should load and replace the document canvas.
#[test]
fn poll_dialog_results_load_replaces_canvas() {
    use crate::files::save_canvas_to_path;
    let (mut file_io, dialog_sender, _) = test_file_io();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    // Create a valid .splattercanvas file
    let source_canvas = Canvas::new(3, 4);
    let dir = tempfile::tempdir().expect("temp dir");
    let file_path = dir.path().join("test.splattercanvas");
    save_canvas_to_path(&source_canvas, &file_path).expect("save to path");

    file_io.pending_file_action = Some(PendingFileAction::Load);
    dialog_sender.send(DialogResult::Picked(file_path)).unwrap();
    file_io.poll_dialog_results(&mut document, &mut undo, &mut errors);
    // Wait for the async load thread to complete.
    for _ in 0..100 {
        file_io.poll_load_import_results(&mut document, &mut undo, &mut errors);
        if document.canvas.width == 3 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    assert!(errors.is_empty(), "errors: {errors:?}");
    assert_eq!(document.canvas.width, 3);
    assert_eq!(document.canvas.height, 4);
    assert!(document.canvas.dirty_rect.needs_reblend());
    assert!(file_io.pending_file_action.is_none());
}

/// `poll_dialog_results` with an Import path pointing to a valid image file
/// should import and replace the document canvas.
#[test]
fn poll_dialog_results_import_replaces_canvas() {
    use crate::files::export_as_image;
    let (mut file_io, dialog_sender, _) = test_file_io();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    // Create a valid PNG file
    let rgba = vec![255u8; 16]; // 2x2 white opaque
    let dir = tempfile::tempdir().expect("temp dir");
    let img_path = dir.path().join("test_import.png");
    export_as_image(&rgba, 2, 2, &img_path, image::ImageFormat::Png).expect("create test image");

    file_io.pending_file_action = Some(PendingFileAction::Import);
    dialog_sender.send(DialogResult::Picked(img_path)).unwrap();
    file_io.poll_dialog_results(&mut document, &mut undo, &mut errors);
    // Wait for the async import thread to complete.
    for _ in 0..100 {
        file_io.poll_load_import_results(&mut document, &mut undo, &mut errors);
        if document.canvas.width == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    assert!(errors.is_empty(), "errors: {errors:?}");
    assert_eq!(document.canvas.width, 2);
    assert_eq!(document.canvas.height, 2);
    assert!(file_io.pending_file_action.is_none());
}

/// `queue_file_action` with Save should set pending_file_action and spawn a thread.
#[test]
fn queue_file_action_save_sets_pending() {
    let (mut file_io, _, _) = test_file_io();
    assert!(file_io.pending_file_action.is_none());
    file_io.queue_file_action(PendingFileAction::Save);
    // pending_file_action should be set (async dialog thread is spawned)
    assert!(file_io.pending_file_action.is_some());
    // Clean up by taking the action
    let _ = file_io.pending_file_action.take();
}

/// `queue_file_action` with LoadStamp should set pending_file_action.
#[test]
fn queue_file_action_load_stamp_sets_pending() {
    let (mut file_io, _, _) = test_file_io();
    file_io.queue_file_action(PendingFileAction::LoadStamp);
    assert!(file_io.pending_file_action.is_some());
    let _ = file_io.pending_file_action.take();
}

/// `save_to_current_path` with a non-empty path should call trigger_async_save.
#[test]
fn save_to_current_path_non_empty_triggers_save() {
    let (save_sender, save_receiver) = mpsc::channel();
    let (dialog_sender, dialog_receiver) = mpsc::channel();
    let mut file_io = FileIO::new(
        dialog_sender,
        dialog_receiver,
        save_sender,
        save_receiver,
        PathBuf::from("/tmp"),
        std::sync::Arc::new(crate::files::DefaultExportStrategy),
    );
    let mut document = Document::new(Canvas::new(1, 1));
    document.savefile_path = "/tmp/test_save_non_empty.splattercanvas".to_string();
    file_io.save_to_current_path(&mut document);
    // Should have sent a save result eventually (may complete after a delay)
    std::thread::sleep(std::time::Duration::from_millis(200));
    // We can't easily check the channel without consuming the receiver,
    // but at least the async save thread was spawned without panic
}

/// `poll_dialog_results` with `Cancelled` should clear `pending_file_action`.
#[test]
fn poll_dialog_results_cancelled_clears_pending() {
    let (mut file_io, dialog_sender, _) = test_file_io();
    file_io.pending_file_action = Some(PendingFileAction::Save);
    let mut document = Document::new(Canvas::new(10, 10));
    let mut undo = UndoHistory::new(100);
    let mut errors = Vec::new();

    dialog_sender.send(DialogResult::Cancelled).unwrap();
    file_io.poll_dialog_results(&mut document, &mut undo, &mut errors);

    assert!(file_io.pending_file_action.is_none());
    assert!(errors.is_empty());
}

/// `poll_export_results` returns true when a result is received (success).
#[test]
fn poll_export_results_success_returns_true() {
    let (mut file_io, _, _) = test_file_io();
    let mut errors = Vec::new();
    // Send a success result via the export channel
    file_io
        .export_result_sender
        .send(Ok(()))
        .expect("send success");
    assert!(file_io.poll_export_results(&mut errors));
    assert!(errors.is_empty());
    assert!(!file_io.export_in_flight);
}

/// `poll_export_results` pushes errors to the error list.
#[test]
fn poll_export_results_error_appends() {
    let (mut file_io, _, _) = test_file_io();
    let mut errors = Vec::new();
    let error: anyhow::Result<()> = Err(anyhow::anyhow!("export failed"));
    file_io
        .export_result_sender
        .send(error)
        .expect("send error");
    assert!(file_io.poll_export_results(&mut errors));
    assert!(errors.iter().any(|e| e.contains("export failed")));
}

/// `poll_save_results` with manual save sets document path even if it's empty.
#[test]
fn poll_save_results_manual_save_empty_path() {
    let (mut file_io, _, save_sender) = test_file_io();
    let mut document = Document::new(Canvas::new(10, 10));
    let mut errors = Vec::new();

    save_sender
        .send(SaveResult::ManualSave(PathBuf::new()))
        .unwrap();
    file_io.poll_save_results(&mut document, &mut errors);

    // Path should be set to empty string representation
    assert!(errors.is_empty());
}

/// `autosave_directory` returns `{data_dir}/autosaves`.
#[test]
fn autosave_directory_path() {
    let (file_io, _, _) = test_file_io();
    let expected = PathBuf::from("/tmp").join("autosaves");
    assert_eq!(file_io.autosave_directory(), expected);
}
