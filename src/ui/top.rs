//! Top menu bar: file operations (new, open, save, save as, export, quit).

use eframe::egui;

use crate::app::MyApp;
use crate::file_io::PendingFileAction;

impl MyApp {
    /// Render the top toolbar with Save, Load, New, Export, Import,
    /// Undo/Redo buttons, and Close.
    ///
    /// Returns `true` if Close was pressed, signaling the app to quit.
    /// Render the top menu bar with Save, Load, New, and Export actions.
    ///
    /// Returns `true` if the user triggered a quit action.
    ///
    /// # Parameters
    ///
    /// * `ui` — The egui UI handle.
    pub fn show_top_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut is_quitting = false;

        // Keyboard shortcuts (checked every frame regardless of button hover).
        if ui.input(|i| i.key_pressed(egui::Key::N) && i.modifiers.command && !i.modifiers.shift) {
            self.ui.show_new_canvas_dialog = true;
        }
        if ui.input(|i| i.key_pressed(egui::Key::O) && i.modifiers.command && !i.modifiers.shift) {
            self.file_io.queue_file_action(PendingFileAction::Load);
            ui.ctx().request_repaint();
        }
        if ui.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command && !i.modifiers.shift) {
            if self.document.savefile_path.is_empty() {
                self.file_io.queue_file_action(PendingFileAction::Save);
            } else {
                self.file_io.save_to_current_path(&self.document);
            }
            ui.ctx().request_repaint();
        }
        if ui.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command && i.modifiers.shift) {
            self.file_io.queue_file_action(PendingFileAction::Save);
            ui.ctx().request_repaint();
        }
        if ui.input(|i| i.key_pressed(egui::Key::I) && i.modifiers.command && !i.modifiers.shift) {
            self.file_io.queue_file_action(PendingFileAction::Import);
            ui.ctx().request_repaint();
        }
        if ui.input(|i| i.key_pressed(egui::Key::E) && i.modifiers.command && !i.modifiers.shift) {
            // Default export: PNG (index 1).
            self.file_io.queue_file_action(PendingFileAction::Export(1));
            ui.ctx().request_repaint();
        }

        ui.horizontal(|ui| {
            // Save
            if ui.button("Save").clicked() {
                if self.document.savefile_path.is_empty() {
                    self.file_io.queue_file_action(PendingFileAction::Save);
                } else {
                    self.file_io.save_to_current_path(&self.document);
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
                self.ui.show_new_canvas_dialog = true;
            }

            // Export menu with all supported formats
            ui.menu_button("Export", |ui| {
                for (format_index, &(label, _)) in crate::app::EXPORT_FORMATS.iter().enumerate() {
                    if ui.button(label).clicked() {
                        self.file_io
                            .queue_file_action(PendingFileAction::Export(format_index));
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
            let undo_button = ui.button("Undo");
            let redo_button = ui.button("Redo");

            // Undo: button or keyboard shortcut
            if self.undo.can_undo()
                && (ui.input(|input_state| {
                    input_state.key_pressed(egui::Key::Z)
                        && input_state.modifiers.command
                        && !input_state.modifiers.shift
                }) || undo_button.clicked())
            {
                self.undo.undo_step(
                    self.document.canvas_mut(),
                    self.ui.undo_redo_steps_multiplier,
                );
                self.document.canvas_mut().render_next_frame = true;
            }

            // Redo: button, cmd+shift+Z, or cmd+Y
            if self.undo.can_redo()
                && (ui.input(|input_state| {
                    input_state.key_pressed(egui::Key::Z)
                        && input_state.modifiers.command
                        && input_state.modifiers.shift
                }) || ui.input(|input_state| {
                    input_state.key_pressed(egui::Key::Y) && input_state.modifiers.command
                }) || redo_button.clicked())
            {
                self.undo.redo_step(
                    self.document.canvas_mut(),
                    self.ui.undo_redo_steps_multiplier,
                );
                self.document.canvas_mut().render_next_frame = true;
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
