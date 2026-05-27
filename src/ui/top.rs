use eframe::egui;

use crate::app::MyApp;
use crate::canvas::Canvas;
use crate::file_io::PendingFileAction;

impl MyApp {
    pub fn show_top_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut is_quitting = false;
        ui.horizontal(|ui| {
            // Save
            if ui.button("Save").clicked() {
                if self.doc.savefile_path.is_empty() {
                    self.file_io.queue_file_action(PendingFileAction::Save);
                } else {
                    self.file_io.save_to_current_path(&self.doc);
                }
                ui.ctx().request_repaint();
            }

            // Load
            if ui.button("Load").clicked() {
                self.file_io.queue_file_action(PendingFileAction::Load);
                ui.ctx().request_repaint();
            }

            // New
            if ui.button("New").clicked() {
                self.doc.replace_canvas(Canvas::default(), &mut self.undo);
                self.tools.previous_tool = None;
                self.tools.previous_cursor_position = None;
            }

            // Export menu with all supported formats
            ui.menu_button("Export", |ui| {
                for (i, &(label, _)) in crate::app::EXPORT_FORMATS.iter().enumerate() {
                    if ui.button(label).clicked() {
                        self.file_io.queue_file_action(PendingFileAction::Export(i));
                        ui.ctx().request_repaint();
                        ui.close();
                    }
                }
            });

            // Import
            if ui.button("Import").clicked() {
                self.file_io.queue_file_action(PendingFileAction::Import);
                ui.ctx().request_repaint();
            }

            ui.separator();

            // Undo / Redo buttons
            let undo_btn = ui.button("Undo");
            let redo_btn = ui.button("Redo");

            // Undo: button or keyboard shortcut
            if
                self.undo.can_undo() &&
                (ui.input(
                    |i| i.key_pressed(egui::Key::Z) && i.modifiers.command && !i.modifiers.shift
                ) || undo_btn.clicked())
            {
                self.undo.undo_step(&mut self.doc.canvas, self.tools.undo_redo_steps_multiplier);
                self.doc.canvas.render_next_frame = true;
            }

            // Redo: button, cmd+shift+Z, or cmd+Y
            if
                self.undo.can_redo() &&
                (ui.input(
                    |i| i.key_pressed(egui::Key::Z) && i.modifiers.command && i.modifiers.shift
                ) ||
                    ui.input(|i| i.key_pressed(egui::Key::Y) && i.modifiers.command) ||
                    redo_btn.clicked())
            {
                self.undo.redo_step(&mut self.doc.canvas, self.tools.undo_redo_steps_multiplier);
                self.doc.canvas.render_next_frame = true;
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
