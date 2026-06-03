//! Tests for application-level constants, UI state defaults, and export
//! format metadata.

use std::path::PathBuf;

use eframe::egui::Color32;

use crate::app::ARCHIVE_EXTENSION;
use crate::app::CANVAS_EXTENSION;
use crate::app::DialogState;
use crate::app::EXPORT_FORMATS;
use crate::app::ErrorState;
use crate::app::ExportInformation;
use crate::app::IMPORT_EXTENSIONS;
use crate::app::NEW_CANVAS_PRESETS;
use crate::app::PendingStamp;
use crate::app::ProgressState;
use crate::app::ToastState;
use crate::app::UIState;
use crate::app::UnsavedWarningAction;
use crate::canvas::RenderState;

/// Default UIState should have IdleThrottled render state, zero elapsed
/// time, no autosaves, no pending layer deletion, and default canvas size.
#[test]
fn ui_state_default_values() {
    let state = UIState::default();
    assert!(
        matches!(state.render_state, RenderState::IdleThrottled),
        "render_state should be IdleThrottled"
    );
    assert!(state.time_elapsed.is_zero());
    assert_eq!(state.times_autosaved, 0);
    assert!(state.last_autosave_time.is_zero());
    assert!(state.errors.list.is_empty());
    assert!(state.dialogs.show_delete_layer_dialog.is_none());
    assert!(!state.dialogs.show_new_canvas_dialog);
    assert_eq!(state.dialogs.new_canvas_width, 2000);
    assert_eq!(state.dialogs.new_canvas_height, 1500);
    assert!(state.dialogs.pending_stamp_name.is_none());
    assert!(state.dialogs.pending_large_canvas.is_none());
    assert!(state.dialogs.pending_unsaved_action.is_none());
    assert!(state.dialogs.pending_after_save.is_none());
    assert!(state.dialogs.pending_brushes.is_none());
    assert!(state.toasts.message.is_none());
}

/// PendingStamp can be constructed with pixel data and dimensions.
#[test]
fn pending_stamp_construction() {
    let pixels = vec![Color32::RED; 4];
    let stamp = PendingStamp {
        pixels,
        width: 2,
        height: 2,
        name: "test_stamp".to_string(),
        spacing: 25,
    };
    assert_eq!(stamp.width, 2);
    assert_eq!(stamp.height, 2);
    assert_eq!(stamp.name, "test_stamp");
    assert_eq!(stamp.pixels.len(), 4);
}

/// All EXPORT_FORMATS entries should have at least one extension.
#[test]
fn export_formats_all_have_extensions() {
    assert!(
        !EXPORT_FORMATS.is_empty(),
        "should have at least one format"
    );
    for (label, info) in EXPORT_FORMATS {
        assert!(
            !info.extensions.is_empty(),
            "format {label} has no extensions"
        );
    }
}

/// EXPORT_FORMATS should reference distinct image formats.
#[test]
fn export_formats_formats_are_distinct() {
    let mut seen = std::collections::HashSet::new();
    for (label, info) in EXPORT_FORMATS {
        assert!(
            seen.insert(info.fmt),
            "duplicate format {label} in EXPORT_FORMATS"
        );
    }
}

/// Each ExportInformation should have a valid image::ImageFormat.
#[test]
fn export_information_extensions() {
    for (label, info) in EXPORT_FORMATS {
        assert!(
            !info.extensions.is_empty(),
            "label {label}: expected at least one extension"
        );
        for ext in info.extensions {
            assert!(!ext.is_empty(), "label {label}: empty extension");
        }
    }
}

/// IMPORT_EXTENSIONS should contain common image file extensions.
#[test]
fn import_extensions_non_empty() {
    assert!(
        !IMPORT_EXTENSIONS.is_empty(),
        "should have import extensions"
    );
    // Check a few expected formats
    let expected = ["png", "jpg", "jpeg", "webp", "gif"];
    for ext in &expected {
        assert!(
            IMPORT_EXTENSIONS.contains(ext),
            "expected {ext} in IMPORT_EXTENSIONS"
        );
    }
}

/// ExportInformation can be read from list entries.
#[test]
fn export_formats_entries_accessible() {
    for (label, info) in EXPORT_FORMATS {
        let _: &str = label;
        let _: &ExportInformation = info;
    }
}

/// `ExportInformation` can be constructed manually.
#[test]
fn export_information_construction() {
    let info = ExportInformation {
        extensions: &["test_ext"],
        fmt: image::ImageFormat::Png,
    };
    assert_eq!(info.extensions, &["test_ext"]);
    assert_eq!(info.fmt, image::ImageFormat::Png);
}

/// `estimate_canvas_memory` accounts for output_rgba + blend buffer + layers.
#[test]
fn estimate_canvas_memory_returns_product() {
    assert_eq!(crate::app::estimate_canvas_memory(100, 100, 1), 120_000);
    assert_eq!(crate::app::estimate_canvas_memory(1, 1, 0), 8);
    assert_eq!(crate::app::estimate_canvas_memory(0, 0, 5), 0);
    assert_eq!(
        crate::app::estimate_canvas_memory(10, 10, 2),
        10 * 10 * 4 * 4
    );
}

/// `DialogState::default()` initialises all fields to their expected defaults.
#[test]
fn dialog_state_default() {
    let state = DialogState::default();
    assert!(state.show_delete_layer_dialog.is_none());
    assert!(!state.show_new_canvas_dialog);
    assert_eq!(state.new_canvas_width, 2000);
    assert_eq!(state.new_canvas_height, 1500);
    assert!(state.pending_large_canvas.is_none());
    assert!(state.pending_stamp_name.is_none());
    assert!(state.pending_brushes.is_none());
    assert!(state.pending_unsaved_action.is_none());
    assert!(state.pending_after_save.is_none());
}

/// `ErrorState::default()` initialises with an empty list.
#[test]
fn error_state_default() {
    let state = ErrorState::default();
    assert!(state.list.is_empty());
}

/// `ToastState::default()` initialises with no message.
#[test]
fn toast_state_default() {
    let state = ToastState::default();
    assert!(state.message.is_none());
}

/// Each `UnsavedWarningAction` variant can be constructed and has stable Debug output.
#[test]
fn unsaved_warning_action_variants() {
    let quit = UnsavedWarningAction::Quit;
    let new_canvas = UnsavedWarningAction::NewCanvas;
    let load = UnsavedWarningAction::Load;
    let import = UnsavedWarningAction::Import;
    let load_path = UnsavedWarningAction::LoadPath(PathBuf::from("/tmp/test.splattercanvas"));

    assert_eq!(format!("{quit:?}"), "Quit");
    assert_eq!(format!("{new_canvas:?}"), "NewCanvas");
    assert_eq!(format!("{load:?}"), "Load");
    assert_eq!(format!("{import:?}"), "Import");
    assert!(format!("{load_path:?}").contains("test.splattercanvas"));
}

/// `ProgressState` variants are distinct and Debug produces variant names.
#[test]
fn progress_state_variants() {
    assert_ne!(ProgressState::Idle, ProgressState::Saving);
    assert_ne!(ProgressState::Autosaving, ProgressState::Exporting);
    assert_ne!(ProgressState::Loading, ProgressState::Importing);
    assert_eq!(format!("{:?}", ProgressState::Idle), "Idle");
    assert_eq!(format!("{:?}", ProgressState::Saving), "Saving");
    assert_eq!(format!("{:?}", ProgressState::Autosaving), "Autosaving");
    assert_eq!(format!("{:?}", ProgressState::Exporting), "Exporting");
    assert_eq!(format!("{:?}", ProgressState::Loading), "Loading");
    assert_eq!(format!("{:?}", ProgressState::Importing), "Importing");
}

/// `NEW_CANVAS_PRESETS` entries have non-empty names and non-zero dimensions.
#[test]
fn new_canvas_presets_are_valid() {
    for (name, width, height) in NEW_CANVAS_PRESETS {
        assert!(!name.is_empty(), "preset name should be non-empty");
        assert!(*width > 0, "preset {name} width should be > 0");
        assert!(*height > 0, "preset {name} height should be > 0");
    }
}

/// App identity and file-extension constants have expected values.
#[test]
fn app_constants_are_correct() {
    assert_eq!(crate::app::APP_QUALIFIER, "com");
    assert_eq!(crate::app::APP_ORGANIZATION, "Monbuticloud");
    assert_eq!(crate::app::APP_NAME, "SplatterIron");
    assert!(CANVAS_EXTENSION.starts_with('.'));
    assert!(ARCHIVE_EXTENSION.starts_with('.'));
    assert_eq!(CANVAS_EXTENSION, ".splattercanvas");
    assert_eq!(ARCHIVE_EXTENSION, ".splatterarchive");
}

/// `PersistedConfig` roundtrips through `serde_json`.
#[test]
fn persisted_config_roundtrip() {
    let cfg = crate::app::PersistedConfig {
        tool_configuration: crate::tool_configuration::ToolConfiguration::default(),
        recent_files: vec![PathBuf::from("/tmp/test.splattercanvas")],
    };
    let json = serde_json::to_string(&cfg).expect("serialize");
    let deserialized: crate::app::PersistedConfig =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(deserialized.recent_files.len(), 1);
    assert_eq!(
        deserialized.recent_files[0],
        PathBuf::from("/tmp/test.splattercanvas")
    );
}
