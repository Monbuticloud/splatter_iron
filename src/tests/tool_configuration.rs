//! Tests for `ToolConfiguration` — default values and field consistency.
//!
//! Confirms that the default tool configuration matches the expected
//! initial state for the application.

use eframe::egui::Color32;

use crate::canvas::CurrentTool;
use crate::tool_configuration::StampTintMode;
use crate::tool_configuration::ToolConfiguration;

/// The default tool configuration should use the Square tool, white color,
/// radius 100, alpha_overlay disabled, and brush preview enabled.
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
}

/// Default stamp/brush tint mode should be Original.
#[test]

fn default_tint_mode_is_original() {

    let config = ToolConfiguration::default();

    assert_eq!(
        config.stamp_config.tint_mode,
        StampTintMode::Original,
        "stamp tint mode defaults to Original"
    );
}
