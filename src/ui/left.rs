//! Left tool palette: tool selection buttons for square, circle, square
//! eraser, circle eraser, bucket-fill, and stamp tool with tint mode.

use eframe::egui;

use crate::app::MyApp;
use crate::canvas::CurrentTool;

/// Selection highlight color for active tool buttons.
/// A deep purple that stands out against both dark and light themes.
const SELECTED_TOOL_COLOR: egui::Color32 = egui::Color32::from_rgb(128, 0, 128);

/// Tint-mode options for the stamp tool.
const STAMP_TINT_MODES: &[(&str, bool)] = &[
    ("Original", false),
    ("Tinted", true),
];

impl MyApp {
    /// Render the left tool panel with selectable tool buttons.
    ///
    /// Temporarily overrides the selection color to purple so the active tool
    /// stands out visually. Shows Square, Circle, Square Eraser, Circle Eraser,
    /// Bucket Fill, and Stamp Tool.
    /// When Stamp is selected, shows stamp info and an Original/Tinted dropdown.
    /// Render the left tool-selection panel (Square, Circle, Erasers, Bucket Fill, Stamp).
    ///
    /// # Parameters
    ///
    /// * `ui` — The egui UI handle.
    pub fn show_left_panel(&mut self, ui: &mut egui::Ui) {
        // Temporarily override selection color to purple for tool buttons.
        // Using ui.selectable_value() gives us built-in highlight + click handling
        // without needing separate button + clicked() checks.
        let old_selection_color = ui.visuals().selection.bg_fill;
        ui.visuals_mut().selection.bg_fill = SELECTED_TOOL_COLOR;

        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::Square,
            "Square Tool"
        );
        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::Circle,
            "Circle Tool"
        );
        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::SquareEraser,
            "Square Eraser"
        );
        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::CircleEraser,
            "Circle Eraser"
        );
        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::BucketFill,
            "Bucket Fill"
        );
        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::Stamp,
            "Stamp Tool"
        );

        // Restore original selection color
        ui.visuals_mut().selection.bg_fill = old_selection_color;

        // Stamp-specific controls
        if self.tool_configuration.current_tool == CurrentTool::Stamp {
            ui.separator();
            if let Some((_, w, h)) = &self.tool_configuration.stamp_image {
                ui.label(format!("Stamp: {w}×{h}"));
                ui.label("Tint mode:");
                for &(label, value) in STAMP_TINT_MODES {
                    ui.selectable_value(&mut self.tool_configuration.stamp_tinted, value, label);
                }
            } else {
                ui.label("No stamp loaded");
                ui.label("Click canvas or right-click");
                ui.label("Replace Stamp Image");
            }
        }
    }
}
