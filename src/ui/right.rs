//! Right panel: color picker, layer stack display, add/delete/move layer
//! buttons, and undo/redo step multiplier slider.

use eframe::egui;

use crate::app::MyApp;
use crate::canvas::CurrentTool;

const UNDO_REDO_RANGE: std::ops::RangeInclusive<usize> = 1..=100;
const BRUSH_RADIUS_RANGE: std::ops::RangeInclusive<u32> = 0..=1000;

/// Internal message enum for deferring layer UI actions.
///
/// Used to collect layer operations (delete, move up/down, select,
/// toggle visibility, rename) during egui widget iteration and apply
/// them afterwards, avoiding borrowing conflicts with the `Document`
/// layer stack.
enum LayerAction {
    /// Delete the layer at the given index.
    Delete(usize),
    /// Move the layer at the given index up one position.
    MoveUp(usize),
    /// Move the layer at the given index down one position.
    MoveDown(usize),
    /// Select the layer at the given index as the current layer.
    Select(usize),
    /// Toggle the visibility of the layer at the given index.
    ToggleVisible(usize),
    /// Rename the layer at the given index.
    Rename(usize, String),
}

impl MyApp {
    /// Render the right settings panel: colour selector, brush radius, alpha
    /// overlay toggle, brush preview toggle, undo strength, layer list,
    /// and layer management controls.
    ///
    /// Layer actions (add, delete, move up/down, select) are processed
    /// via a `LayerAction` enum to avoid borrowing conflicts.
    ///
    /// # Parameters
    ///
    /// * `ui` — The egui UI handle.
    pub fn show_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Settings");
        ui.separator();
        ui.label("Color Selector");
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    self.tool_configuration.current_tool == CurrentTool::Eyedropper,
                    "Eyedropper",
                )
                .clicked()
            {
                self.tool_configuration.current_tool = CurrentTool::Eyedropper;
            }
            ui.color_edit_button_srgba(&mut self.tool_configuration.current_color);
        });

        ui.separator();

        ui.label("Undo/Redo Strength");
        ui.add(
            egui::DragValue::new(&mut self.ui.undo_redo_steps_multiplier).range(UNDO_REDO_RANGE),
        )
        .on_hover_text("Number of paint strokes per undo or redo step (max 100)");

        ui.label("::Brush Settings::");
        ui.separator();
        ui.label("Brush Radius:");
        ui.add(egui::DragValue::new(&mut self.tool_configuration.radius).range(BRUSH_RADIUS_RANGE));
        ui.checkbox(
            &mut self.tool_configuration.show_brush_preview,
            "Brush Preview",
        );
        ui.checkbox(&mut self.tool_configuration.alpha_overlay, "Alpha Overlay");
        ui.checkbox(&mut self.tool_configuration.show_grid, "Show Grid");
        ui.add_enabled(
            self.tool_configuration.show_grid,
            egui::DragValue::new(&mut self.tool_configuration.grid_size)
                .range(1..=500)
                .prefix("Grid: "),
        );
        ui.checkbox(
            &mut self.tool_configuration.stabilization_enabled,
            "Stabilize",
        );
        ui.add_enabled(
            self.tool_configuration.stabilization_enabled,
            egui::Slider::new(
                &mut self.tool_configuration.stabilization_smoothing,
                0.0..=100.0,
            )
            .text("Smoothing"),
        )
        .on_hover_text("Higher values make the virtual cursor lag further behind the real cursor");

        ui.separator();
        ui.label("Save Path:");
        ui.add(
            egui::TextEdit::singleline(&mut self.document.savefile_path)
                .id_source("save_path_text"),
        );

        ui.separator();
        let add_layer_button = ui.button("Add Layer");
        if add_layer_button.clicked() {
            self.document.add_layer(&mut self.undo);
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut pending_layer_action: Option<LayerAction> = None;

            // Opacity changes need to be applied after the loop; collect them here.
            let mut pending_opacity: Option<(usize, u8)> = None;

            for (i, layer) in self.document.canvas.pixels.iter().enumerate() {
                let header_label = if layer.name.is_empty() {
                    format!("Layer {i}")
                } else {
                    layer.name.clone()
                };

                ui.horizontal(|ui| {
                    // Visibility toggle
                    let vis_label = if layer.visible { "👁" } else { "⛔" };
                    if ui.selectable_label(false, vis_label).clicked() {
                        pending_layer_action = Some(LayerAction::ToggleVisible(i));
                    }

                    // Layer header (collapsing)
                    let response = egui::CollapsingHeader::new(header_label).show(ui, |ui| {
                        // Editable name
                        let mut name = layer.name.clone();
                        let resp = ui.text_edit_singleline(&mut name);
                        if resp.lost_focus() && name != layer.name {
                            pending_layer_action = Some(LayerAction::Rename(i, name));
                        }

                        // Opacity slider
                        let mut opacity = layer.opacity;
                        if ui
                            .add(egui::Slider::new(&mut opacity, 0..=255).text("Opacity"))
                            .changed()
                        {
                            pending_opacity = Some((i, opacity));
                        }

                        // Action buttons
                        let delete_button = ui.button("Delete");
                        if delete_button.clicked() {
                            pending_layer_action = Some(LayerAction::Delete(i));
                        }

                        let move_up_button = ui.button("Move Up");
                        if move_up_button.clicked() && i > 0 {
                            pending_layer_action = Some(LayerAction::MoveUp(i));
                        }

                        let move_down_button = ui.button("Move Down");
                        if move_down_button.clicked() && i < self.document.canvas.pixels.len() - 1 {
                            pending_layer_action = Some(LayerAction::MoveDown(i));
                        }

                        let select_button = ui.button("Select");
                        if select_button.clicked() {
                            pending_layer_action = Some(LayerAction::Select(i));
                        }
                        if i == self.document.current_layer {
                            ui.label("Currently Selected");
                        }
                    });
                    // Click on the header selects the layer
                    if response.header_response.clicked() {
                        pending_layer_action = Some(LayerAction::Select(i));
                    }
                });
            }

            // Apply opacity change (needs mutable access to canvas).
            if let Some((index, opacity)) = pending_opacity {
                self.document
                    .set_layer_opacity(index, opacity, &mut self.undo);
            }

            // Apply deferred layer actions.
            if let Some(layer_action) = pending_layer_action {
                match layer_action {
                    LayerAction::Delete(index) => {
                        if self.document.canvas.pixels.len() > 1 {
                            self.ui.dialogs.show_delete_layer_dialog = Some(index);
                        }
                    }
                    LayerAction::MoveUp(index) => {
                        if index > 0 {
                            self.document.move_layer_up(index, &mut self.undo);
                        }
                    }
                    LayerAction::MoveDown(index) => {
                        if index < self.document.canvas.pixels.len() - 1 {
                            self.document.move_layer_down(index, &mut self.undo);
                        }
                    }
                    LayerAction::Select(index) => {
                        self.document.select_layer(index);
                    }
                    LayerAction::ToggleVisible(index) => {
                        self.document.toggle_layer_visible(index, &mut self.undo);
                    }
                    LayerAction::Rename(index, new_name) => {
                        self.document.rename_layer(index, new_name, &mut self.undo);
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::UNDO_REDO_RANGE;
    use super::BRUSH_RADIUS_RANGE;
    use super::LayerAction;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn undo_redo_range_start_less_than_or_equal_end() {
        assert!(UNDO_REDO_RANGE.start() <= UNDO_REDO_RANGE.end());
        assert!(*UNDO_REDO_RANGE.start() >= 1);
        assert!(*UNDO_REDO_RANGE.end() <= 100);
    }

    #[test]
    fn brush_radius_range_start_less_than_or_equal_end() {
        assert!(BRUSH_RADIUS_RANGE.start() <= BRUSH_RADIUS_RANGE.end());
        assert!(*BRUSH_RADIUS_RANGE.end() <= 1000);
    }

    #[test]
    fn layer_action_delete_holds_index() {
        let action = LayerAction::Delete(42);
        if let LayerAction::Delete(index) = action {
            assert_eq!(index, 42);
        } else {
            panic!("expected Delete variant");
        }
    }

    #[test]
    fn layer_action_move_up_holds_index() {
        let action = LayerAction::MoveUp(1);
        if let LayerAction::MoveUp(index) = action {
            assert_eq!(index, 1);
        } else {
            panic!("expected MoveUp variant");
        }
    }

    #[test]
    fn layer_action_move_down_holds_index() {
        let action = LayerAction::MoveDown(2);
        if let LayerAction::MoveDown(index) = action {
            assert_eq!(index, 2);
        } else {
            panic!("expected MoveDown variant");
        }
    }

    #[test]
    fn layer_action_select_holds_index() {
        let action = LayerAction::Select(3);
        if let LayerAction::Select(index) = action {
            assert_eq!(index, 3);
        } else {
            panic!("expected Select variant");
        }
    }

    #[test]
    fn layer_action_toggle_visible_holds_index() {
        let action = LayerAction::ToggleVisible(4);
        if let LayerAction::ToggleVisible(index) = action {
            assert_eq!(index, 4);
        } else {
            panic!("expected ToggleVisible variant");
        }
    }

    #[test]
    fn layer_action_rename_holds_index_and_name() {
        let action = LayerAction::Rename(5, "test".into());
        if let LayerAction::Rename(index, name) = action {
            assert_eq!(index, 5);
            assert_eq!(name, "test");
        } else {
            panic!("expected Rename variant");
        }
    }

    #[test]
    fn show_right_panel_renders_settings_headers() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_right_panel(ui);
        });
        harness.step();

        let _settings = harness.get_by_label("Settings");
        let _color = harness.get_by_label("Color Selector");
        let _brush = harness.get_by_label("::Brush Settings::");
        let _add = harness.get_by_label("Add Layer");
    }

    #[test]
    fn show_right_panel_renders_layer_zero() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_right_panel(ui);
        });
        harness.step();

        // Default layer shows "Layer 1" (1-indexed display)
        harness.get_by_label("Layer 1");
    }
}
