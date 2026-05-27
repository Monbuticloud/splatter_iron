use eframe::egui::Color32;

use crate::canvas::CurrentTool;

pub struct ToolConfig {
    pub current_tool: CurrentTool,
    pub current_color: Color32,
    pub radius: u32,
    pub previous_tool: Option<CurrentTool>,
    pub previous_cursor_position: Option<(u32, u32)>,
    pub show_brush_preview: bool,
    pub undo_redo_steps_multiplier: usize,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            current_tool: CurrentTool::Square,
            current_color: Color32::from_rgba_premultiplied(255, 255, 255, 255),
            radius: 100,
            previous_tool: None,
            previous_cursor_position: None,
            show_brush_preview: true,
            undo_redo_steps_multiplier: 5,
        }
    }
}
