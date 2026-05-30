//! Test modules for all crate functionality.
//!
//! Each module mirrors a corresponding source module to validate its
//! correctness, edge cases, and invariants under controlled conditions.

pub mod app;
pub mod asset_library;
pub mod brush_common;
pub mod brush_library;
pub mod brush_params;
pub mod brush_parsers;
pub mod bucket_fill;
pub mod canvas;
pub mod circle_brush;
pub mod common;
pub mod custom_brush;
pub mod debug;
pub mod document;
pub mod file_io;
pub mod files;
pub mod pixel;
pub mod square_brush;
pub mod stamp_brush;
pub mod stamp_library;
pub mod tool_configuration;
pub mod persistence;
pub mod undo;
pub mod undo_history;
