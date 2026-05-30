//! Brush and fill tool implementations.

/// Shared brush utilities (visited-pixel run capture).
pub mod brush_common;
/// `.abr`, `.gbr`, and `.brush` file format parsers.
pub mod brush_parsers;
/// Scanline flood-fill tool implementation.
pub mod bucket_fill;
/// Midpoint-circle brush tool implementation.
pub mod circle_brush;
/// Custom brush line drawing from loaded brush tips.
pub mod custom_brush;
/// Rectangular brush tool implementation.
pub mod square_brush;
/// External-image stamp brush tool implementation.
pub mod stamp_brush;
