//! Current tool, color, brush radius, alpha-overlay toggle, undo/redo step
//! multiplier, and transient UI interaction state (cursor position, drag).

use eframe::egui::Color32;

use crate::canvas::CurrentTool;

/// Current tool selection, color, brush radius, undo/redo step count, and UI interaction state.
pub struct ToolConfiguration {
    /// The currently selected drawing tool.
    pub current_tool: CurrentTool,
    /// Color applied by brush strokes (premultiplied-alpha).
    pub current_color: Color32,
    /// Brush radius in pixels.
    pub radius: u32,
    /// Whether strokes use alpha-overlay blending instead of opaque.
    pub alpha_overlay: bool,
    /// Tool selected before the current one (used for eraser toggle-back).
    pub previous_tool: Option<CurrentTool>,
    /// Cursor position from the previous frame (used for brush preview).
    pub previous_cursor_position: Option<(u32, u32)>,
    /// Whether to show the brush size preview circle/square on the canvas.
    pub show_brush_preview: bool,
    /// Multiplier applied to undo/redo step count during fast-scroll.
    pub undo_redo_steps_multiplier: usize,
}

impl Default for ToolConfiguration {
    /// Create a default tool configuration with the Square tool selected,
    /// white color, radius 100, brush preview enabled, and undo/redo step
    /// multiplier of 5.
    fn default() -> Self {
        Self {
            current_tool: CurrentTool::Square,
            current_color: Color32::from_rgba_premultiplied(255, 255, 255, 255),
            radius: 100,
            alpha_overlay: false,
            previous_tool: None,
            previous_cursor_position: None,
            show_brush_preview: true,
            undo_redo_steps_multiplier: 1,
        }
    }
}
