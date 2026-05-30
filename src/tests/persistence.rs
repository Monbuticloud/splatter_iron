//! Tests for config persistence — recent files, save/load round-trips,
//! and the autosave-timer guard.
//!
//! Validates that `push_recent_file` maintains order/cap invariants,
//! that `save_config` writes a valid JSON file on disk, and that
//! `handle_config_save` respects the autosave interval.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use eframe::egui::Color32;

use crate::app::MyApp;
use crate::app::PersistedConfig;
use crate::app::UIState;
use crate::asset_library::Library;
use crate::canvas::Canvas;
use crate::canvas::CurrentTool;
use crate::document::Document;
use crate::file_io::FileIO;
use crate::files::DefaultExportStrategy;
use crate::tool_configuration::ToolConfiguration;
use crate::undo_history::UndoHistory;

/// Build a `MyApp` rooted at `data_dir` so persistence methods write
/// into the temporary directory. The returned `TempDir` must stay alive
/// for the lifetime of the app.
fn app_with_temp_data_dir() -> (MyApp, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let data_dir = dir.path().to_path_buf();
    let canvas = Canvas::new(10, 10);
    let pixel_count = (canvas.width * canvas.height) as usize;
    let (dialog_tx, dialog_rx) = mpsc::channel();
    let (save_tx, save_rx) = mpsc::channel();

    let app = MyApp {
        document: Document::new(canvas),
        tool_configuration: ToolConfiguration::default(),
        undo: UndoHistory::new(pixel_count),
        file_io: FileIO::new(
            dialog_tx,
            dialog_rx,
            save_tx,
            save_rx,
            data_dir.clone(),
            Arc::new(DefaultExportStrategy),
        ),
        ui: UIState::default(),
        gpu_texture: None,
        stamp_library: Library::load_from_disk(&data_dir),
        brush_library: Library::load_from_disk(&data_dir),
    };

    (app, dir)
}

// --- push_recent_file ---

/// Adding an empty path is a no-op.
#[test]
fn push_recent_file_empty_path_noop() {
    let (mut app, _dir) = app_with_temp_data_dir();
    assert!(app.ui.recent_files.is_empty());

    app.push_recent_file(PathBuf::new());
    assert!(app.ui.recent_files.is_empty());
}

/// Adding a path inserts it at position 0.
#[test]
fn push_recent_file_inserts_front() {
    let (mut app, _dir) = app_with_temp_data_dir();
    app.push_recent_file(PathBuf::from("/a"));
    app.push_recent_file(PathBuf::from("/b"));

    assert_eq!(app.ui.recent_files.len(), 2);
    assert_eq!(app.ui.recent_files[0], PathBuf::from("/b"));
    assert_eq!(app.ui.recent_files[1], PathBuf::from("/a"));
}

/// Adding a duplicate moves it to position 0 and removes the old entry.
#[test]
fn push_recent_file_dedup() {
    let (mut app, _dir) = app_with_temp_data_dir();
    app.push_recent_file(PathBuf::from("/a"));
    app.push_recent_file(PathBuf::from("/b"));
    app.push_recent_file(PathBuf::from("/a"));

    assert_eq!(app.ui.recent_files.len(), 2);
    assert_eq!(app.ui.recent_files[0], PathBuf::from("/a"));
    assert_eq!(app.ui.recent_files[1], PathBuf::from("/b"));
}

/// Pushing 11 paths truncates to 10.
#[test]
fn push_recent_file_truncates_at_ten() {
    let (mut app, _dir) = app_with_temp_data_dir();
    for i in 0..11 {
        app.push_recent_file(PathBuf::from(format!("/path_{i}")));
    }
    assert_eq!(app.ui.recent_files.len(), 10);
    // Most recent is /path_10, oldest remaining is /path_1
    assert_eq!(app.ui.recent_files[0], PathBuf::from("/path_10"));
    assert_eq!(app.ui.recent_files[9], PathBuf::from("/path_1"));
}

// --- config_path ---

/// `config_path` returns a path ending in `config.json` inside the data dir.
#[test]
fn config_path_ends_with_config_json() {
    let (app, dir) = app_with_temp_data_dir();
    let path = app.config_path();

    assert!(path.ends_with("config.json"));
    // Parent directory should be the data dir
    assert_eq!(path.parent(), Some(dir.path()));
}

// --- save_config ---

/// `save_config` writes a valid JSON file that can be deserialized back
/// to the same tool configuration and recent files list.
#[test]
fn save_config_roundtrip() {
    let (mut app, dir) = app_with_temp_data_dir();
    app.tool_configuration.current_tool = CurrentTool::Circle;
    app.tool_configuration.current_color = Color32::RED;
    app.tool_configuration.radius = 50;
    app.push_recent_file(PathBuf::from("/recent/file.splattercanvas"));

    app.save_config();

    let config_path = dir.path().join("config.json");
    assert!(config_path.exists(), "config.json should exist");

    let json = std::fs::read_to_string(&config_path).expect("read config.json");
    let loaded: PersistedConfig = serde_json::from_str(&json).expect("deserialize config");

    assert_eq!(loaded.tool_configuration.current_tool, CurrentTool::Circle);
    assert_eq!(loaded.tool_configuration.current_color, Color32::RED);
    assert_eq!(loaded.tool_configuration.radius, 50);
    assert_eq!(
        loaded.recent_files,
        vec![PathBuf::from("/recent/file.splattercanvas")]
    );
}

/// `save_config` with an empty recent-files list still writes a valid file.
#[test]
fn save_config_empty_recent_files() {
    let (app, dir) = app_with_temp_data_dir();
    assert!(app.ui.recent_files.is_empty());

    app.save_config();

    let config_path = dir.path().join("config.json");
    assert!(config_path.exists());
    let json = std::fs::read_to_string(&config_path).expect("read config.json");
    let loaded: PersistedConfig = serde_json::from_str(&json).expect("deserialize config");
    assert!(loaded.recent_files.is_empty());
}

// --- handle_config_save ---

/// `handle_config_save` does not save before the autosave interval elapses.
#[test]
fn handle_config_save_waits_for_interval() {
    let (mut app, dir) = app_with_temp_data_dir();
    // Immediately after construction, last_autosave_time == time_elapsed == 0,
    // so the interval check should NOT trigger a save.
    let config_path = dir.path().join("config.json");
    if config_path.exists() {
        std::fs::remove_file(&config_path).expect("remove config before test");
    }
    app.handle_config_save();
    assert!(
        !config_path.exists(),
        "should not save before interval elapses"
    );
}

/// `handle_config_save` saves after the autosave interval elapses.
#[test]
fn handle_config_save_saves_after_interval() {
    let (mut app, dir) = app_with_temp_data_dir();
    let config_path = dir.path().join("config.json");
    if config_path.exists() {
        std::fs::remove_file(&config_path).expect("remove config before test");
    }
    // Advance time past the interval
    app.ui.time_elapsed = Duration::from_secs(121); // 2 min + 1 s
    app.handle_config_save();
    assert!(config_path.exists(), "should save after interval elapses");
}
