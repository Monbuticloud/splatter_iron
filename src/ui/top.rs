use eframe::egui;

use crate::app::{ MyApp, PendingFileAction };
use crate::canvas::Canvas;
use crate::undo::{ undo_apply, redo_apply };

impl MyApp {
    pub fn show_top_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut is_quitting = false;
        ui.horizontal(|ui| {
            // Save
            if ui.button("Save").clicked() {
                if self.savefile_path.is_empty() {
                    self.queue_file_action(PendingFileAction::Save);
                } else {
                    self.save_to_current_path();
                }
                ui.ctx().request_repaint();
            }

            // Load
            if ui.button("Load").clicked() {
                self.queue_file_action(PendingFileAction::Load);
                ui.ctx().request_repaint();
            }

            // New
            if ui.button("New").clicked() {
                self.replace_canvas(Canvas::default());
            }

            // Export menu with all supported formats
            ui.menu_button("Export", |ui| {
                for (i, &(label, _)) in crate::app::EXPORT_FORMATS.iter().enumerate() {
                    if ui.button(label).clicked() {
                        self.queue_file_action(PendingFileAction::Export(i));
                        ui.ctx().request_repaint();
                        ui.close();
                    }
                }
            });

            // Import
            if ui.button("Import").clicked() {
                self.queue_file_action(PendingFileAction::Import);
                ui.ctx().request_repaint();
            }

            ui.separator();

            // Undo / Redo buttons
            let undo_btn = ui.button("Undo");
            let redo_btn = ui.button("Redo");

            // Undo: button or keyboard shortcut
            if
                self.redo_index < self.stroke_stack.len() &&
                (ui.input(
                    |i| i.key_pressed(egui::Key::Z) && i.modifiers.command && !i.modifiers.shift
                ) || undo_btn.clicked())
            {
                let count = self.undo_redo_steps_multiplier.min(
                    self.stroke_stack.len() - self.redo_index
                );
                for _ in 0..count {
                    let idx = self.stroke_stack.len() - 1 - self.redo_index;
                    undo_apply(&mut self.canvas, &self.stroke_stack[idx]);
                    self.redo_index += 1;
                }
                self.canvas.render_next_frame = true;
            }

            // Redo: button, cmd+shift+Z, or cmd+Y
            if
                self.redo_index > 0 &&
                (ui.input(
                    |i| i.key_pressed(egui::Key::Z) && i.modifiers.command && i.modifiers.shift
                ) ||
                    ui.input(|i| i.key_pressed(egui::Key::Y) && i.modifiers.command) ||
                    redo_btn.clicked())
            {
                let count = self.undo_redo_steps_multiplier.min(self.redo_index);
                for _ in 0..count {
                    let idx = self.stroke_stack.len() - self.redo_index;
                    self.redo_index -= 1;
                    redo_apply(&mut self.canvas, &self.stroke_stack[idx]);
                }
                self.canvas.render_next_frame = true;
            }

            ui.separator();

            // Close
            if ui.button("Close").clicked() {
                is_quitting = true;
            }
        });
        is_quitting
    }
}
