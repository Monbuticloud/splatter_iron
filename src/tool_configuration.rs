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

/// Shared sampling configuration for stamp and brush-tip rendering.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]

pub struct SamplingConfig {
    /// Pixel-sampling strategy (nearest or bilinear).
    pub sampling: StampSampling,
    /// Whether to tint by the current tool color.
    pub tint_mode: StampTintMode,
}

impl Default for SamplingConfig {
    fn default() -> Self {

        Self {
            sampling: StampSampling::Nearest,
            tint_mode: StampTintMode::Original,
        }
    }
}

/// Current tool selection, color, brush radius, and stamp/brush sampling config.
#[derive(Clone, Debug, Serialize, Deserialize)]

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
    /// Sampling configuration for stamp tool rendering.
    pub stamp_config: SamplingConfig,
    /// Sampling configuration for custom brush tip rendering.
    pub brush_config: SamplingConfig,
    /// Whether the pixel-grid overlay is visible on the canvas.
    pub show_grid: bool,
    /// Grid spacing in canvas pixels.
    pub grid_size: u32,
    /// Whether brush stabilization (lerped virtual cursor) is enabled.
    pub stabilization_enabled: bool,
    /// Smoothing strength for brush stabilization (0 = snappy, 100 = frozen).
    pub stabilization_smoothing: f32,
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
            stamp_config: SamplingConfig::default(),
            brush_config: SamplingConfig::default(),
            show_grid: false,
            grid_size: 50,
            stabilization_enabled: false,
            stabilization_smoothing: 30.0,
        }
    }
}
