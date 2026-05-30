//! Top menu bar: file operations (new, open, save, save as, export, quit).

use eframe::egui;

use crate::app::MyApp;
use crate::app::UnsavedWarningAction;
use crate::file_io::PendingFileAction;

impl MyApp {
    /// Render the top menu bar with Save, Load, New, Export, Import,
    /// Undo/Redo buttons, keyboard shortcuts, and Close.
    ///
    /// Returns `true` if the user triggered a quit action.
    ///
    /// # Parameters
    ///
    /// * `ui` — The egui UI handle.
    pub fn show_top_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut is_quitting = false;

        // Keyboard shortcuts for tool switching (single keys, no modifier).
        let is_in_text_field = ui.memory(|m| {
            if m.has_focus(egui::Id::new("save_path_text"))
                || m.has_focus(egui::Id::new("stamp_name_text"))
            {
                return true;
            }
            // Check brush name fields (dynamic IDs up to 64 brushes).
            (0..64).any(|i| m.has_focus(egui::Id::new(format!("brush_name_{i}"))))
        });
        if !is_in_text_field {
            use crate::canvas::CurrentTool;
            if ui.input(|i| i.key_pressed(egui::Key::S) && !i.modifiers.command) {
                self.tool_configuration.current_tool = CurrentTool::Square;
            }
            if ui.input(|i| i.key_pressed(egui::Key::C) && !i.modifiers.command) {
                self.tool_configuration.current_tool = CurrentTool::Circle;
            }
            if ui.input(|i| i.key_pressed(egui::Key::E) && !i.modifiers.command && !i.modifiers.shift)
            {
                let is_eraser = matches!(
                    self.tool_configuration.current_tool,
                    CurrentTool::SquareEraser | CurrentTool::CircleEraser
                );
                if is_eraser {
                    if let Some(prev) = self.ui.previous_tool.take() {
                        self.tool_configuration.current_tool = prev;
                    }
                } else {
                    self.ui.previous_tool = Some(self.tool_configuration.current_tool);
                    self.tool_configuration.current_tool = CurrentTool::SquareEraser;
                }
            }
            if ui.input(|i| i.key_pressed(egui::Key::E) && i.modifiers.shift && !i.modifiers.command) {
                let is_eraser = matches!(
                    self.tool_configuration.current_tool,
                    CurrentTool::SquareEraser | CurrentTool::CircleEraser
                );
                if !is_eraser {
                    self.ui.previous_tool = Some(self.tool_configuration.current_tool);
                }
                self.tool_configuration.current_tool = CurrentTool::CircleEraser;
            }
            if ui.input(|i| i.key_pressed(egui::Key::G) && !i.modifiers.command) {
                self.tool_configuration.current_tool = CurrentTool::BucketFill;
            }
            if ui.input(|i| i.key_pressed(egui::Key::T) && !i.modifiers.command) {
                self.tool_configuration.current_tool = CurrentTool::Stamp;
            }
            if ui.input(|i| i.key_pressed(egui::Key::B) && !i.modifiers.command) {
                self.tool_configuration.current_tool = CurrentTool::CustomBrush;
            }
            if ui.input(|i| i.key_pressed(egui::Key::I) && !i.modifiers.command) {
                self.tool_configuration.current_tool = CurrentTool::Eyedropper;
            }
            if ui.input(|i| i.key_pressed(egui::Key::H) && !i.modifiers.command) {
                self.tool_configuration.current_tool = CurrentTool::Pan;
            }
        }

        // Keyboard shortcuts (checked every frame regardless of button hover).
        if ui.input(|i| i.key_pressed(egui::Key::N) && i.modifiers.command && !i.modifiers.shift) {
            self.guard_unsaved(UnsavedWarningAction::NewCanvas);
        }
        if ui.input(|i| i.key_pressed(egui::Key::O) && i.modifiers.command && !i.modifiers.shift) {
            self.guard_unsaved(UnsavedWarningAction::Load);
        }
        if ui.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command && !i.modifiers.shift) {
            if self.document.savefile_path.is_empty() {
                self.file_io.queue_file_action(PendingFileAction::Save);
            } else {
                self.file_io.save_to_current_path(&mut self.document);
            }
            ui.ctx().request_repaint();
        }
        if ui.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command && i.modifiers.shift) {
            self.file_io.queue_file_action(PendingFileAction::Save);
            ui.ctx().request_repaint();
        }
        if ui.input(|i| i.key_pressed(egui::Key::I) && i.modifiers.command && !i.modifiers.shift) {
            self.guard_unsaved(UnsavedWarningAction::Import);
        }
        if ui.input(|i| i.key_pressed(egui::Key::E) && i.modifiers.command && !i.modifiers.shift) {
            self.file_io
                .queue_file_action(PendingFileAction::Export(self.ui.last_export_format));
            ui.ctx().request_repaint();
        }

        ui.horizontal(|ui| {
            // Save
            if ui.button("Save").clicked() {
                if self.document.savefile_path.is_empty() {
                    self.file_io.queue_file_action(PendingFileAction::Save);
                } else {
                    self.file_io.save_to_current_path(&mut self.document);
                }
                ui.ctx().request_repaint();
            }

            // Load
            if ui.button("Load").clicked() {
                self.guard_unsaved(UnsavedWarningAction::Load);
                ui.ctx().request_repaint();
            }

            // New
            if ui.button("New").clicked() {
                self.guard_unsaved(UnsavedWarningAction::NewCanvas);
            }

            // Export menu with all supported formats
            ui.menu_button("Export", |ui| {
                for (format_index, &(label, _)) in crate::app::EXPORT_FORMATS.iter().enumerate() {
                    if ui.button(label).clicked() {
                        self.ui.last_export_format = format_index;
                        self.file_io
                            .queue_file_action(PendingFileAction::Export(format_index));
                        ui.ctx().request_repaint();
                        ui.close();
                    }
                }
            });

            // Recent files
            let has_recent = !self.ui.recent_files.is_empty();
            let recent_response = ui.add_enabled(has_recent, egui::Button::new("Recent"));
            if has_recent {
                recent_response.context_menu(|ui| {
                    for path in &self.ui.recent_files {
                        if ui.button(path.display().to_string()).clicked() {
                            self.guard_unsaved(UnsavedWarningAction::LoadPath(path.clone()));
                            ui.ctx().request_repaint();
                            ui.close();
                        }
                    }
                });
            }

            // Import
            if ui.button("Import").clicked() {
                self.guard_unsaved(UnsavedWarningAction::Import);
                ui.ctx().request_repaint();
            }

            // Autosaves — open the autosave folder in the OS file manager
            if ui.button("Autosaves").clicked() {
                let path = self.file_io.autosave_directory();
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&path).spawn();
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("explorer").arg(&path).spawn();
                #[cfg(all(unix, not(target_os = "macos")))]
                let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
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
                self.document.canvas_mut().dirty_rect.request_full_blend();
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
                self.document.canvas_mut().dirty_rect.request_full_blend();
            }

            ui.separator();

            // Close
            if ui.button("Close").clicked() {
                self.guard_unsaved(UnsavedWarningAction::Quit);
                if !self.document.dirty_since_last_autosave {
                    is_quitting = true;
                }
            }
        });
        is_quitting
    }
}
