//! Right panel: color picker, layer stack display, add/delete/move layer
//! buttons, and undo/redo step multiplier slider.

use eframe::egui;

use crate::app::MyApp;

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
                    let response = egui::CollapsingHeader::new(header_label)
                        .show(ui, |ui| {
                            // Editable name
                            let mut name = layer.name.clone();
                            let resp = ui.text_edit_singleline(&mut name);
                            if resp.lost_focus() && name != layer.name {
                                pending_layer_action =
                                    Some(LayerAction::Rename(i, name));
                            }

                            // Opacity slider
                            let mut opacity = layer.opacity;
                            if ui
                                .add(
                                    egui::Slider::new(&mut opacity, 0..=255)
                                        .text("Opacity"),
                                )
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
                            if move_down_button.clicked()
                                && i < self.document.canvas.pixels.len() - 1
                            {
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
                let canvas = self.document.canvas_mut();
                canvas.pixels[index].opacity = opacity;
                canvas.dirty_rect.request_full_blend();
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
                            self.document.move_layer_up(index);
                        }
                    }
                    LayerAction::MoveDown(index) => {
                        if index < self.document.canvas.pixels.len() - 1 {
                            self.document.move_layer_down(index);
                        }
                    }
                    LayerAction::Select(index) => {
                        self.document.select_layer(index);
                    }
                    LayerAction::ToggleVisible(index) => {
                        let canvas = self.document.canvas_mut();
                        if let Some(l) = canvas.pixels.get_mut(index) {
                            l.visible = !l.visible;
                            canvas.dirty_rect.request_full_blend();
                        }
                    }
                    LayerAction::Rename(index, new_name) => {
                        if let Some(l) = self.document.canvas_mut().pixels.get_mut(index) {
                            l.name = new_name;
                        }
                    }
                }
            }
        });
    }
}
