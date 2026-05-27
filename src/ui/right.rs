use eframe::egui;

use crate::app::MyApp;

const UNDO_REDO_RANGE: std::ops::RangeInclusive<usize> = 1..=1000;
const BRUSH_RADIUS_RANGE: std::ops::RangeInclusive<u32> = 0..=350;

enum LayerAction {
    Delete(usize),
    MoveUp(usize),
    MoveDown(usize),
    Select(usize),
}

impl MyApp {
    /// Render the right settings panel with colour picker, brush settings,
    /// undo/redo strength, save path, and layer management controls.
    ///
    /// Layer actions (add, delete, move up/down, select) are processed
    /// via a `LayerAction` enum to avoid borrowing conflicts.
    pub fn show_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Settings");
        ui.separator();
        ui.label("Color Selector");

        ui.color_edit_button_srgba(&mut self.tools.current_color);

        ui.separator();

        ui.label("Undo/Redo Strength");
        ui.add(egui::DragValue::new(&mut self.tools.undo_redo_steps_multiplier).range(UNDO_REDO_RANGE));

        ui.label("::Brush Settings::");
        ui.separator();
        ui.label("Brush Radius:");
        ui.add(egui::DragValue::new(&mut self.tools.radius).range(BRUSH_RADIUS_RANGE));
        ui.checkbox(&mut self.tools.show_brush_preview, "Brush Preview");

        ui.separator();
        ui.label("Save Path:");
        ui.text_edit_singleline(&mut self.doc.savefile_path);

        ui.separator();
        let add_layer_button = ui.button("Add Layer");
        if add_layer_button.clicked() {
            self.doc.add_layer();
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut pending_layer_action = None;
            for (i, _layer) in self.doc.canvas.pixels.iter().enumerate() {
                let _layer_panel = egui::CollapsingHeader
                    ::new(format!("Layer {i}"))
                    .show(ui, |ui| {
                        let delete_button = ui.button("Delete");
                        if delete_button.clicked() {
                            pending_layer_action = Some(LayerAction::Delete(i));
                        }

                        let move_up_button = ui.button("Move Up");
                        if move_up_button.clicked() && i > 0 {
                            pending_layer_action = Some(LayerAction::MoveUp(i));
                        }

                        let move_down_button = ui.button("Move Down");
                        if move_down_button.clicked() && i < self.doc.canvas.pixels.len() - 1 {
                            pending_layer_action = Some(LayerAction::MoveDown(i));
                        }

                        let select_button = ui.button("Select");
                        if select_button.clicked() {
                            pending_layer_action = Some(LayerAction::Select(i));
                        }
                        if i == self.doc.current_layer {
                            ui.label("Currently Selected");
                        }
                    });
            }
            if let Some(layer_action) = pending_layer_action {
                match layer_action {
                    LayerAction::Delete(index) => {
                        if
                            self.ui.pending_layer_for_deletion == Some(index) &&
                            self.doc.canvas.pixels.len() > 1
                        {
                            self.ui.pending_layer_for_deletion = None;
                            self.doc.delete_layer(index);
                        } else {
                            self.ui.pending_layer_for_deletion = Some(index);
                        }
                    }
                    LayerAction::MoveUp(index) => {
                        if index > 0 {
                            self.ui.pending_layer_for_deletion = None;
                            self.doc.move_layer_up(index);
                        }
                    }
                    LayerAction::MoveDown(index) => {
                        if index < self.doc.canvas.pixels.len() - 1 {
                            self.ui.pending_layer_for_deletion = None;
                            self.doc.move_layer_down(index);
                        }
                    }
                    LayerAction::Select(index) => {
                        self.doc.select_layer(index);
                    }
                }
            }
        });
    }
}
