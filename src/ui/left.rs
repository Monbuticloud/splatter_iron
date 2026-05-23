use eframe::egui;

use crate::app::MyApp;
use crate::canvas::CurrentTool;

impl MyApp {
    #[inline(always)]
    pub fn show_left_panel(&mut self, ui: &mut egui::Ui) {
        let square_paint_tool_button = ui.button("Square Tool");
        if square_paint_tool_button.clicked() {
            self.current_tool = CurrentTool::SquareTool;
        }
        let circle_paint_tool_button = ui.button("Circle Tool");
        if circle_paint_tool_button.clicked() {
            self.current_tool = CurrentTool::CircleTool;
        }
        let square_eraser_tool_button = ui.button("Square Eraser");
        if square_eraser_tool_button.clicked() {
            self.current_tool = CurrentTool::SquareEraserTool;
        }
        let circle_eraser_tool_button = ui.button("Circle Eraser");
        if circle_eraser_tool_button.clicked() {
            self.current_tool = CurrentTool::CircleEraserTool;
        }
    }
}