//! Tests for the [`BrushStrokeParams`] parameter bundle.
//!
//! Verifies construction, field access, and Debug output.

use eframe::egui::Color32;

use crate::brush_params::BrushStrokeParams;
use crate::canvas::Canvas;
use crate::canvas::DirtyRectList;
use crate::canvas::Layer;

/// Helper: build a minimal canvas fixture.
fn canvas() -> Canvas {
    Canvas {
        pixels: vec![Layer {
            pixels: vec![Color32::TRANSPARENT; 100],
            ..Default::default()
        }],
        height: 10,
        width: 10,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
    }
}

/// Constructing BrushStrokeParams and verifying all fields are accessible.
#[test]
fn construction_and_field_access() {
    let mut c = canvas();
    let mut visited = vec![0u32; 100];
    let mut drag = vec![0u32; 100];

    let params = BrushStrokeParams {
        start_x: 1,
        start_y: 2,
        end_x: 5,
        end_y: 6,
        canvas: &mut c,
        color: Color32::RED,
        layer: 0,
        visited: &mut visited,
        stamp: 42,
        alpha_overlay: true,
        drag_processed: &mut drag,
        drag_stamp_value: 7,
    };

    assert_eq!(params.start_x, 1);
    assert_eq!(params.start_y, 2);
    assert_eq!(params.end_x, 5);
    assert_eq!(params.end_y, 6);
    assert_eq!(params.color, Color32::RED);
    assert_eq!(params.layer, 0);
    assert_eq!(params.stamp, 42);
    assert!(params.alpha_overlay);
    assert_eq!(params.drag_stamp_value, 7);
}

/// Debug output contains key field information.
#[test]
fn debug_output() {
    let mut c = canvas();
    let mut visited = vec![0u32; 100];
    let mut drag = vec![0u32; 100];

    let params = BrushStrokeParams {
        start_x: 0,
        start_y: 0,
        end_x: 3,
        end_y: 4,
        canvas: &mut c,
        color: Color32::BLUE,
        layer: 0,
        visited: &mut visited,
        stamp: 1,
        alpha_overlay: false,
        drag_processed: &mut drag,
        drag_stamp_value: 0,
    };

    let debug = format!("{params:?}");
    assert!(debug.contains("start_x: 0"), "debug should include start_x");
    assert!(debug.contains("end_y: 4"), "debug should include end_y");
    assert!(debug.contains("visited.len: 100"), "debug should include visited.len");
    assert!(debug.contains("drag_processed.len: 100"), "debug should include drag_processed.len");
}
