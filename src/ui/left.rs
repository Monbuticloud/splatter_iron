use eframe::egui;

use crate::app::MyApp;
use crate::canvas::CurrentTool;

/// Selection highlight color for active tool buttons.
/// A deep purple that stands out against both dark and light themes.
const SELECTED_TOOL_COLOR: egui::Color32 = egui::Color32::from_rgb(128, 0, 128);

impl MyApp {
    #[inline(always)]
    pub fn show_left_panel(&mut self, ui: &mut egui::Ui) {
        // Temporarily override selection color to purple for tool buttons.
        // Using ui.selectable_value() gives us built-in highlight + click handling
        // without needing separate button + clicked() checks.
        let old_selection_color = ui.visuals().selection.bg_fill;
        ui.visuals_mut().selection.bg_fill = SELECTED_TOOL_COLOR;

        ui.selectable_value(&mut self.current_tool, CurrentTool::SquareTool, "Square Tool");
        ui.selectable_value(&mut self.current_tool, CurrentTool::CircleTool, "Circle Tool");
        ui.selectable_value(&mut self.current_tool, CurrentTool::SquareEraserTool, "Square Eraser");
        ui.selectable_value(&mut self.current_tool, CurrentTool::CircleEraserTool, "Circle Eraser");

        // Restore original selection color
        ui.visuals_mut().selection.bg_fill = old_selection_color;
    }
}