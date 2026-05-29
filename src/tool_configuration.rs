//! Current tool, color, brush radius, alpha-overlay toggle, and stamp/brush
//! sampling configuration.

use eframe::egui::Color32;
use serde::Deserialize;
use serde::Serialize;

use crate::canvas::CurrentTool;

/// Pixel-sampling strategy when scaling a stamp or brush tip to canvas size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StampSampling {
    /// Nearest-neighbour (sharp edges, pixel-art friendly).
    Nearest,
    /// Bilinear interpolation (smooth scaling for photographs).
    Bilinear,
}

/// Tint mode for stamp and brush-tip rendering.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StampTintMode {
    /// Use the tip's original colours.
    Original,
    /// Multiply tip pixels by `current_color`.
    Tinted,
}

/// Current tool selection, color, brush radius, and stamp/brush sampling config.
#[derive(Serialize, Deserialize)]
pub struct ToolConfiguration {
    /// The currently selected drawing tool.
    pub current_tool: CurrentTool,
    /// Color applied by brush strokes (premultiplied-alpha).
    pub current_color: Color32,
    /// Brush radius in pixels.
    pub radius: u32,
    /// Whether strokes use alpha-overlay blending instead of opaque.
    pub alpha_overlay: bool,
    /// Whether to show the brush size preview circle/square on the canvas.
    pub show_brush_preview: bool,
    /// Sampling strategy when scaling the stamp to canvas size.
    pub stamp_sampling: StampSampling,
    /// Whether stamp pixels are tinted by `current_color`.
    pub stamp_tint_mode: StampTintMode,
    /// Sampling strategy when scaling a custom brush tip to canvas size.
    pub brush_sampling: StampSampling,
    /// Whether custom brush tip pixels are tinted by `current_color`.
    pub brush_tint_mode: StampTintMode,
}

impl Default for ToolConfiguration {
    /// Create a default tool configuration with the Square tool selected,
    /// white color, radius 100, brush preview enabled.
    fn default() -> Self {
        Self {
            current_tool: CurrentTool::Square,
            current_color: Color32::from_rgba_premultiplied(255, 255, 255, 255),
            radius: 100,
            alpha_overlay: false,
            show_brush_preview: true,
            stamp_sampling: StampSampling::Nearest,
            stamp_tint_mode: StampTintMode::Original,
            brush_sampling: StampSampling::Nearest,
            brush_tint_mode: StampTintMode::Original,
        }
    }
}
