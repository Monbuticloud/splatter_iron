//! Tests for `ToolConfiguration` — default values and field consistency.
//!
//! Confirms that the default tool configuration matches the expected
//! initial state for the application.

use eframe::egui::Color32;

use crate::canvas::CurrentTool;
use crate::tool_configuration::ToolConfiguration;

/// The default tool configuration should use the Square tool, white color,
/// radius 100, alpha_overlay disabled, brush preview enabled, and
/// undo_redo_steps_multiplier of 1.
#[test]
fn default_values_match_expected() {
    let config = ToolConfiguration::default();
    assert_eq!(config.current_tool, CurrentTool::Square, "default tool");
    assert_eq!(
        config.current_color,
        Color32::from_rgba_premultiplied(255, 255, 255, 255),
        "default color"
    );
    assert_eq!(config.radius, 100, "default radius");
    assert!(!config.alpha_overlay, "alpha overlay defaults to false");
    assert!(config.show_brush_preview, "brush preview defaults to true");
    assert_eq!(config.undo_redo_steps_multiplier, 1, "undo/redo multiplier");
}

/// The default configuration should have no previous tool and no cursor position.
#[test]
fn default_optional_fields_are_none() {
    let config = ToolConfiguration::default();
    assert!(config.previous_tool.is_none(), "no previous tool");
    assert!(
        config.previous_cursor_position.is_none(),
        "no cursor position"
    );
    assert!(config.stamp_image.is_none(), "no stamp image");
    assert!(!config.stamp_tinted, "stamp tinted defaults to false");
}
