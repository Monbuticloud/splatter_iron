//! Shared test helpers: small canvases, premultiplied-color shorthands.
//!
//! Used by all other test modules to reduce boilerplate when constructing
//! predictable input state.

use eframe::egui::Color32;

use crate::canvas::{ Canvas, Layer };

/// Build a 10×10 single-layer transparent canvas for use in tests.
///
/// # Returns
///
/// A pre-built `Canvas` with one fully transparent layer at 10×10 resolution.
pub fn small_canvas() -> Canvas {
    Canvas {
        pixels: vec![Layer {
            pixels: vec![Color32::TRANSPARENT; 100],
        }],
        height: 10,
        width: 10,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: None,
        render_next_frame: false,
    }
}

/// Shortcut for a fully opaque red color in premultiplied format.
///
/// # Returns
///
/// `Color32::from_rgba_premultiplied(255, 0, 0, 255)`.
pub fn red() -> Color32 {
    Color32::from_rgba_premultiplied(255, 0, 0, 255)
}

/// Shortcut for a fully opaque blue color in premultiplied format.
///
/// # Returns
///
/// `Color32::from_rgba_premultiplied(0, 0, 255, 255)`.
pub fn blue() -> Color32 {
    Color32::from_rgba_premultiplied(0, 0, 255, 255)
}
