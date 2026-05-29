//! Tests for application-level constants, UI state defaults, and export
//! format metadata.

use eframe::egui::Color32;

use crate::app::EXPORT_FORMATS;
use crate::app::ExportInformation;
use crate::app::IMPORT_EXTENSIONS;
use crate::app::PendingStamp;
use crate::app::UIState;
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
    assert!(state.dialogs.pending_layer_for_deletion.is_none());
    assert!(!state.dialogs.show_new_canvas_dialog);
    assert_eq!(state.dialogs.new_canvas_width, 2000);
    assert_eq!(state.dialogs.new_canvas_height, 1500);
    assert!(state.dialogs.pending_stamp_name.is_none());
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
