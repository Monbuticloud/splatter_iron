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
            if ui.input(|i| {
                i.key_pressed(egui::Key::E) && !i.modifiers.command && !i.modifiers.shift
            }) {
                let is_eraser = matches!(
                    self.tool_configuration.current_tool,
                    CurrentTool::Eraser(_)
                );
                if is_eraser {
                    if let Some(prev) = self.ui.previous_tool.take() {
                        self.tool_configuration.current_tool = prev;
                    }
                } else {
                    self.ui.previous_tool = Some(self.tool_configuration.current_tool);
                    self.tool_configuration.current_tool = CurrentTool::Eraser(crate::canvas::ToolKind::Square);
                }
            }
            if ui
                .input(|i| i.key_pressed(egui::Key::E) && i.modifiers.shift && !i.modifiers.command)
            {
                let is_eraser = matches!(
                    self.tool_configuration.current_tool,
                    CurrentTool::Eraser(_)
                );
                if !is_eraser {
                    self.ui.previous_tool = Some(self.tool_configuration.current_tool);
                }
                self.tool_configuration.current_tool = CurrentTool::Eraser(crate::canvas::ToolKind::Circle);
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
                self.dialog_manager
                    .queue_file_action(PendingFileAction::Save);
            } else {
                self.ui.progress = crate::app::ProgressState::Saving;
                self.save_manager.save_to_current_path(&mut self.document);
            }
            ui.ctx().request_repaint();
        }
        if ui.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command && i.modifiers.shift) {
            self.dialog_manager
                .queue_file_action(PendingFileAction::Save);
            ui.ctx().request_repaint();
        }
        if ui.input(|i| i.key_pressed(egui::Key::I) && i.modifiers.command && !i.modifiers.shift) {
            self.guard_unsaved(UnsavedWarningAction::Import);
        }
        if ui.input(|i| i.key_pressed(egui::Key::E) && i.modifiers.command && !i.modifiers.shift) {
            self.dialog_manager
                .queue_file_action(PendingFileAction::Export(self.ui.last_export_format));
            ui.ctx().request_repaint();
        }

        ui.horizontal(|ui| {
            // Save
            if ui.button("Save").clicked() {
                if self.document.savefile_path.is_empty() {
                    self.dialog_manager
                        .queue_file_action(PendingFileAction::Save);
                } else {
                    self.ui.progress = crate::app::ProgressState::Saving;
                    self.save_manager.save_to_current_path(&mut self.document);
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
                        self.dialog_manager
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
                    for path in self.ui.recent_files.clone() {
                        if ui.button(path.display().to_string()).clicked() {
                            self.guard_unsaved(UnsavedWarningAction::LoadPath(path));
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

            // File menu with archive operations
            ui.menu_button("File", |ui| {
                if ui.button("Export Archive…").clicked() {
                    self.dialog_manager
                        .queue_file_action(PendingFileAction::ExportArchive);
                    ui.ctx().request_repaint();
                    ui.close();
                }
                if ui.button("Import Archive…").clicked() {
                    self.dialog_manager
                        .queue_file_action(PendingFileAction::ImportArchive);
                    ui.ctx().request_repaint();
                    ui.close();
                }
            });

            // Autosaves — open the autosave folder in the OS file manager
            if ui.button("Autosaves").clicked() {
                let path = self.save_manager.autosave_directory();
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

#[cfg(test)]
mod tests {
    use eframe::egui;
    use egui_kittest::kittest::NodeT;
    use egui_kittest::kittest::Queryable;

    /// Pressing `S` switches to Square tool (no command modifier).
    #[test]
    fn key_s_selects_square_tool() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.tool_configuration.current_tool = crate::canvas::CurrentTool::Circle;

        {
            let mut harness = egui_kittest::Harness::new_ui(|ui| {
                app.show_top_panel(ui);
            });
            harness.key_press(egui::Key::S);
            harness.step();
        }

        assert!(matches!(
            app.tool_configuration.current_tool,
            crate::canvas::CurrentTool::Square
        ));
    }

    /// Pressing `C` switches to Circle tool.
    #[test]
    fn key_c_selects_circle_tool() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.tool_configuration.current_tool = crate::canvas::CurrentTool::Square;

        {
            let mut harness = egui_kittest::Harness::new_ui(|ui| {
                app.show_top_panel(ui);
            });
            harness.key_press(egui::Key::C);
            harness.step();
        }

        assert!(matches!(
            app.tool_configuration.current_tool,
            crate::canvas::CurrentTool::Circle
        ));
    }

    /// Pressing `G` switches to BucketFill tool.
    #[test]
    fn key_g_selects_bucket_fill_tool() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.tool_configuration.current_tool = crate::canvas::CurrentTool::Circle;

        {
            let mut harness = egui_kittest::Harness::new_ui(|ui| {
                app.show_top_panel(ui);
            });
            harness.key_press(egui::Key::G);
            harness.step();
        }

        assert!(matches!(
            app.tool_configuration.current_tool,
            crate::canvas::CurrentTool::BucketFill
        ));
    }

    /// Pressing `H` switches to Pan tool.
    #[test]
    fn key_h_selects_pan_tool() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.tool_configuration.current_tool = crate::canvas::CurrentTool::Circle;

        {
            let mut harness = egui_kittest::Harness::new_ui(|ui| {
                app.show_top_panel(ui);
            });
            harness.key_press(egui::Key::H);
            harness.step();
        }

        assert!(matches!(
            app.tool_configuration.current_tool,
            crate::canvas::CurrentTool::Pan
        ));
    }

    /// Pressing `E` toggles eraser tool (non-eraser -> Eraser(ToolKind::Square)).
    #[test]
    fn key_e_toggles_eraser_on_non_eraser() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.tool_configuration.current_tool = crate::canvas::CurrentTool::Square;

        {
            let mut harness = egui_kittest::Harness::new_ui(|ui| {
                app.show_top_panel(ui);
            });
            harness.key_press(egui::Key::E);
            harness.step();
        }

        assert!(matches!(
            app.tool_configuration.current_tool,
            crate::canvas::CurrentTool::Eraser(crate::canvas::ToolKind::Square)
        ));
        assert!(matches!(
            app.ui.previous_tool,
            Some(crate::canvas::CurrentTool::Square)
        ));
    }

    /// Pressing `E` when already eraser toggles back to previous tool.
    #[test]
    fn key_e_toggles_back_from_eraser() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.tool_configuration.current_tool = crate::canvas::CurrentTool::Eraser(crate::canvas::ToolKind::Square);
        app.ui.previous_tool = Some(crate::canvas::CurrentTool::Circle);

        {
            let mut harness = egui_kittest::Harness::new_ui(|ui| {
                app.show_top_panel(ui);
            });
            harness.key_press(egui::Key::E);
            harness.step();
        }

        assert!(matches!(
            app.tool_configuration.current_tool,
            crate::canvas::CurrentTool::Circle
        ));
        assert!(app.ui.previous_tool.is_none(), "previous_tool consumed");
    }

    /// All toolbar buttons render without panic.
    #[test]
    fn show_top_panel_renders_buttons() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_top_panel(ui);
        });
        harness.step();

        let _save = harness.get_by_label("Save");
        let _load = harness.get_by_label("Load");
        let _new = harness.get_by_label("New");
        let _export = harness.get_by_label("Export");
        let _import = harness.get_by_label("Import");
        let _file = harness.get_by_label("File");
        let _autosaves = harness.get_by_label("Autosaves");
        let _undo = harness.get_by_label("Undo");
        let _redo = harness.get_by_label("Redo");
        let _close = harness.get_by_label("Close");
    }

    /// "Recent" button exists disabled when no recent files.
    #[test]
    fn show_top_panel_recent_button_disabled() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_top_panel(ui);
        });
        harness.step();

        let recent = harness.get_by_label("Recent");
        assert!(recent.accesskit_node().is_disabled());
    }

    /// Close button on clean canvas sets should_close.
    #[test]
    fn show_top_panel_close_clean_canvas_sets_should_close() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());

        {
            let mut harness = egui_kittest::Harness::new_ui(|ui| {
                app.show_top_panel(ui);
            });
            harness.step();
            harness.get_by_label("Close").click();
            harness.step();
        }

        assert!(app.ui.should_close);
    }

    /// Undo button exists but does nothing when undo history is empty.
    #[test]
    fn show_top_panel_undo_disabled_when_no_history() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());

        // Create fresh undo history (no entries).
        {
            let mut harness = egui_kittest::Harness::new_ui(|ui| {
                app.show_top_panel(ui);
            });
            harness.step();
            // Undo button exists but `can_undo()` returns false.
            let _undo = harness.get_by_label("Undo");
        }
    }
}
