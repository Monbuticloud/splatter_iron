//! Right panel: color picker, layer stack display, add/delete/move layer
//! buttons, and undo/redo step multiplier slider.

use eframe::egui;

use crate::app::MyApp;

const UNDO_REDO_RANGE: std::ops::RangeInclusive<usize> = 1..=1000;
const BRUSH_RADIUS_RANGE: std::ops::RangeInclusive<u32> = 0..=1000;

/// Internal message enum for deferring layer UI actions.
///
/// Used to collect layer operations (delete, move up/down, select) during
/// egui widget iteration and apply them afterwards, avoiding borrowing
/// conflicts with the `Document` layer stack.
enum LayerAction {
    /// Delete the layer at the given index.
    Delete(usize),
    /// Move the layer at the given index up one position.
    MoveUp(usize),
    /// Move the layer at the given index down one position.
    MoveDown(usize),
    /// Select the layer at the given index as the current layer.
    Select(usize),
}

impl MyApp {
    /// Render the right settings panel with color picker, brush settings,
    /// undo/redo strength, save path, and layer management controls.
    ///
    /// Layer actions (add, delete, move up/down, select) are processed
    /// via a `LayerAction` enum to avoid borrowing conflicts.
    /// Render the right settings panel: colour selector, brush radius, alpha
    /// overlay toggle, brush preview toggle, undo strength, layer list, and save path.
    ///
    /// # Parameters
    ///
    /// * `ui` — The egui UI handle.
    pub fn show_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Settings");
        ui.separator();
        ui.label("Color Selector");

        ui.color_edit_button_srgba(&mut self.tool_configuration.current_color);

        ui.separator();

        ui.label("Undo/Redo Strength");
        ui.add(
            egui::DragValue::new(&mut self.ui.undo_redo_steps_multiplier)
                .range(UNDO_REDO_RANGE),
        )
        .on_hover_text("Number of paint strokes per undo or redo step");

        ui.label("::Brush Settings::");
        ui.separator();
        ui.label("Brush Radius:");
        ui.add(egui::DragValue::new(&mut self.tool_configuration.radius).range(BRUSH_RADIUS_RANGE));
        ui.checkbox(
            &mut self.tool_configuration.show_brush_preview,
            "Brush Preview",
        );
        ui.checkbox(&mut self.tool_configuration.alpha_overlay, "Alpha Overlay");

        ui.separator();
        ui.label("Save Path:");
        ui.add(
            egui::TextEdit::singleline(&mut self.document.savefile_path)
                .id_source("save_path_text"),
        );

        ui.separator();
        let add_layer_button = ui.button("Add Layer");
        if add_layer_button.clicked() {
            self.document.add_layer();
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut pending_layer_action = None;
            for (i, _layer) in self.document.canvas.pixels.iter().enumerate() {
                let _layer_panel =
                    egui::CollapsingHeader::new(format!("Layer {i}")).show(ui, |ui| {
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
            }
            if let Some(layer_action) = pending_layer_action {
                match layer_action {
                    LayerAction::Delete(index) => {
                        if self.ui.dialogs.pending_layer_for_deletion == Some(index)
                            && self.document.canvas.pixels.len() > 1
                        {
                            self.ui.dialogs.pending_layer_for_deletion = None;
                            self.document.delete_layer(index);
                        } else {
                            self.ui.dialogs.pending_layer_for_deletion = Some(index);
                        }
                    }
                    LayerAction::MoveUp(index) => {
                        if index > 0 {
                            self.ui.dialogs.pending_layer_for_deletion = None;
                            self.document.move_layer_up(index);
                        }
                    }
                    LayerAction::MoveDown(index) => {
                        if index < self.document.canvas.pixels.len() - 1 {
                            self.ui.dialogs.pending_layer_for_deletion = None;
                            self.document.move_layer_down(index);
                        }
                    }
                    LayerAction::Select(index) => {
                        self.document.select_layer(index);
                    }
                }
            }
        });
    }
}
