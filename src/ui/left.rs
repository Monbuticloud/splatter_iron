use eframe::egui;

use crate::app::MyApp;
use crate::canvas::CurrentTool;

/// Selection highlight color for active tool buttons.
/// A deep purple that stands out against both dark and light themes.
const SELECTED_TOOL_COLOR: egui::Color32 = egui::Color32::from_rgb(128, 0, 128);

impl MyApp {
    /// Render the left tool panel with selectable tool buttons.
    ///
    /// Temporarily overrides the selection colour to purple for visual
    /// distinction. Shows Square, Circle, Square Eraser, and Circle Eraser.
    pub fn show_left_panel(&mut self, ui: &mut egui::Ui) {
        // Temporarily override selection color to purple for tool buttons.
        // Using ui.selectable_value() gives us built-in highlight + click handling
        // without needing separate button + clicked() checks.
        let old_selection_color = ui.visuals().selection.bg_fill;
        ui.visuals_mut().selection.bg_fill = SELECTED_TOOL_COLOR;

        ui.selectable_value(&mut self.tools.current_tool, CurrentTool::Square, "Square Tool");
        ui.selectable_value(&mut self.tools.current_tool, CurrentTool::Circle, "Circle Tool");
        ui.selectable_value(&mut self.tools.current_tool, CurrentTool::SquareEraser, "Square Eraser");
        ui.selectable_value(&mut self.tools.current_tool, CurrentTool::CircleEraser, "Circle Eraser");

        // Restore original selection color
        ui.visuals_mut().selection.bg_fill = old_selection_color;
    }
}
