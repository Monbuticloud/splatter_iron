use eframe::egui::{ self, Color32 };

use crate::app::MyApp;
use crate::canvas::Layer;

enum LayerAction {
    Delete(usize),
    MoveUp(usize),
    MoveDown(usize),
    Select(usize),
}

impl MyApp {
    #[inline(always)]
    pub fn show_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Settings");
        ui.separator();
        ui.label("Color Selector");

        ui.color_edit_button_srgba(&mut self.current_color);

        ui.separator();

        ui.label("Undo/Redo Strength");
        ui.add(egui::DragValue::new(&mut self.undo_redo_strength).range(1..=1000));

        ui.label("::Brush Settings::");
        ui.separator();
        ui.label("Brush Radius:");
        ui.add(egui::DragValue::new(&mut self.radius).range(0..=350));
        ui.checkbox(&mut self.show_brush_preview, "Brush Preview");

        ui.separator();
        ui.label("Save Path:");
        ui.text_edit_singleline(&mut self.savefile_path);

        ui.separator();
        let _current_layer_text = format!("Current Layer: {}", self.current_layer);

        let add_layer_button = ui.button("Add Layer");
        if add_layer_button.clicked() {
            self.canvas.pixels.push(Layer {
                pixels: vec![Color32::TRANSPARENT; (self.canvas.width * self.canvas.height) as usize],
            });
            // self.canvas.render_next_frame = true;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut pending_layer_action = None;
            for (i, _layer) in self.canvas.pixels.iter().enumerate() {
                let _layer_panel = egui::CollapsingHeader
                    ::new(format!("Layer {}", i))
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
                        if move_down_button.clicked() && i < self.canvas.pixels.len() - 1 {
                            pending_layer_action = Some(LayerAction::MoveDown(i));
                        }

                        let select_button = ui.button("Select");
                        if select_button.clicked() {
                            pending_layer_action = Some(LayerAction::Select(i));
                        }
                        if i == self.current_layer {
                            ui.label("Currently Selected");
                        }
                    });
            }
            if let Some(layer_action) = pending_layer_action {
                match layer_action {
                    LayerAction::Delete(index) => {
                        if self.pending_delete_layer == Some(index) && self.canvas.pixels.len() > 1 {
                            self.pending_delete_layer = None;
                            self.canvas.pixels.remove(index);
                            self.current_layer = self.current_layer
                                .saturating_sub(1)
                                .min(self.canvas.pixels.len() - 1);
                            self.canvas.render_next_frame = true;
                        } else {
                            self.pending_delete_layer = Some(index);
                        }
                    }
                    LayerAction::MoveUp(index) => {
                        if index > 0 {
                            self.canvas.pixels.swap(index, index - 1);
                            self.canvas.render_next_frame = true;
                            self.current_layer = index - 1;
                        }
                    }
                    LayerAction::MoveDown(index) => {
                        if index < self.canvas.pixels.len() - 1 {
                            self.canvas.pixels.swap(index, index + 1);
                            self.canvas.render_next_frame = true;
                            self.current_layer = index + 1;
                        }
                    }
                    LayerAction::Select(index) => {
                        self.current_layer = index;
                    }
                }
            }
        });
    }
}
