//! Dialog windows: error list, large-canvas warning, delete-layer
//! confirmation, new-canvas presets, unsaved-changes warning, stamp naming,
//! brush naming, toast notifications, and progress indicator.

use std::time::Instant;

use eframe::egui;

use crate::app::MEMORY_WARNING_THRESHOLD;
use crate::app::MyApp;
use crate::app::NEW_CANVAS_PRESETS;
use crate::app::PendingStamp;
use crate::app::UnsavedWarningAction;
use crate::app::estimate_canvas_memory;
use crate::canvas::Canvas;
use crate::file_io::PendingFileAction;

impl MyApp {
    /// Show the error-list window (dismiss, copy, dismiss-all).
    pub(crate) fn show_error_window(&mut self, ui: &mut egui::Ui) {
        if self.ui.errors.list.is_empty() {
            return;
        }
        let mut open = true;
        let mut to_dismiss: Vec<usize> = Vec::new();
        egui::Window::new("Error")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                for (index, msg) in self.ui.errors.list.iter().enumerate() {
                    ui.label(format!("Error: {msg}"));
                    ui.horizontal(|ui| {
                        if ui.button("Dismiss").clicked() {
                            to_dismiss.push(index);
                        }
                        if ui.button("Copy error").clicked() {
                            ui.ctx().copy_text(msg.clone());
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("Dismiss All").clicked() {
                        to_dismiss.extend(0..self.ui.errors.list.len());
                    }
                });
            });

        to_dismiss.sort_unstable_by(|a, b| b.cmp(a));
        to_dismiss.dedup();
        for i in to_dismiss {
            self.ui.errors.list.remove(i);
        }
        if !open {
            self.ui.errors.list.clear();
        }
    }

    /// Show the "Large Canvas Warning" confirmation dialog.
    pub(crate) fn show_large_canvas_warning(&mut self, ui: &mut egui::Ui) {
        let Some((w, h)) = self.ui.dialogs.pending_large_canvas else {
            return;
        };
        let mut open = true;
        let estimated = estimate_canvas_memory(w, h, 1);
        let estimated_mb = estimated / (1024 * 1024);
        egui::Window::new("Large Canvas Warning")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.label(format!(
                    "This canvas ({w}×{h}) may use up to ~{estimated_mb} MB of RAM.\n\
                     Proceed? This cannot be undone."
                ));
                ui.horizontal(|ui| {
                    if ui.button("Yes, create").clicked() {
                        let canvas = Canvas::new(w, h);
                        self.document.replace_canvas(canvas, &mut self.undo);
                        self.ui.previous_tool = None;
                        self.ui.previous_cursor_position = None;
                        self.ui.dialogs.show_new_canvas_dialog = false;
                        self.ui.dialogs.pending_large_canvas = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.ui.dialogs.pending_large_canvas = None;
                    }
                });
            });
        if !open {
            self.ui.dialogs.pending_large_canvas = None;
        }
    }

    /// Show the "Delete Layer" confirmation dialog.
    pub(crate) fn show_delete_layer_dialog(&mut self, ui: &mut egui::Ui) {
        let Some(index) = self.ui.dialogs.show_delete_layer_dialog else {
            return;
        };
        let layer_name = self
            .document
            .canvas
            .pixels
            .get(index)
            .map(|l| {
                if l.name.is_empty() {
                    format!("Layer {index}")
                } else {
                    l.name.clone()
                }
            })
            .unwrap_or_default();
        let mut open = true;
        egui::Window::new("Delete Layer")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.label(format!("Delete \"{layer_name}\"? This cannot be undone."));
                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        self.document.delete_layer(index, &mut self.undo);
                        self.ui.dialogs.show_delete_layer_dialog = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.ui.dialogs.show_delete_layer_dialog = None;
                    }
                });
            });
        if !open {
            self.ui.dialogs.show_delete_layer_dialog = None;
        }
    }

    /// Show the "New Canvas" preset / custom-size dialog.
    pub(crate) fn show_new_canvas_dialog(&mut self, ui: &mut egui::Ui) {
        if !self.ui.dialogs.show_new_canvas_dialog {
            return;
        }
        let mut open = true;
        egui::Window::new("New Canvas")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for &(label, width, height) in NEW_CANVAS_PRESETS {
                        if ui.button(format!("{label}\n{width}×{height}")).clicked() {
                            self.ui.dialogs.new_canvas_width = width;
                            self.ui.dialogs.new_canvas_height = height;
                        }
                    }
                });
                ui.separator();
                ui.label("Custom:");
                ui.add(
                    egui::Slider::new(
                        &mut self.ui.dialogs.new_canvas_width,
                        4..=self.ui.max_texture_dimension,
                    )
                    .text("Width"),
                );
                ui.add(
                    egui::Slider::new(
                        &mut self.ui.dialogs.new_canvas_height,
                        4..=self.ui.max_texture_dimension,
                    )
                    .text("Height"),
                );
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        if self.document.dirty_since_last_autosave {
                            self.ui.dialogs.pending_unsaved_action =
                                Some(UnsavedWarningAction::NewCanvas);
                        } else {
                            let mem = estimate_canvas_memory(
                                self.ui.dialogs.new_canvas_width,
                                self.ui.dialogs.new_canvas_height,
                                1,
                            );
                            if mem > MEMORY_WARNING_THRESHOLD {
                                self.ui.dialogs.pending_large_canvas = Some((
                                    self.ui.dialogs.new_canvas_width,
                                    self.ui.dialogs.new_canvas_height,
                                ));
                            } else {
                                let canvas = Canvas::new(
                                    self.ui.dialogs.new_canvas_width,
                                    self.ui.dialogs.new_canvas_height,
                                );
                                self.document.replace_canvas(canvas, &mut self.undo);
                                self.ui.previous_tool = None;
                                self.ui.previous_cursor_position = None;
                                self.ui.dialogs.show_new_canvas_dialog = false;
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.ui.dialogs.show_new_canvas_dialog = false;
                    }
                });
            });
        if !open {
            self.ui.dialogs.show_new_canvas_dialog = false;
        }
    }

    /// Show the unsaved-changes warning when a destructive action is triggered
    /// while the canvas has unsaved modifications.
    ///
    /// Offers Save (save then proceed), Discard (lose changes and proceed),
    /// and Cancel (do nothing). The deferred action is stored in
    /// `pending_unsaved_action` and cleared on resolution.
    pub(crate) fn show_unsaved_changes_warning(&mut self, ui: &mut egui::Ui) {
        let Some(action) = self.ui.dialogs.pending_unsaved_action.clone() else {
            return;
        };
        let mut open = true;
        let mut resolved = false;
        let label: String = if self.document.savefile_path.is_empty() {
            "You have unsaved changes. What would you like to do?".to_string()
        } else {
            format!(
                "\"{}\" has unsaved changes. What would you like to do?",
                std::path::Path::new(&self.document.savefile_path)
                    .file_name()
                    .map(|s| s.to_string_lossy())
                    .unwrap_or_default()
            )
        };
        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.label(&label);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        if self.document.savefile_path.is_empty() {
                            self.ui.dialogs.pending_after_save = Some(action.clone());
                            self.dialog_manager.queue_file_action(PendingFileAction::Save);
                        } else {
                            self.save_manager.save_to_current_path(&mut self.document);
                            self.ui.dialogs.pending_after_save = Some(action.clone());
                        }
                        resolved = true;
                    }
                    if !resolved && ui.button("Don't Save").clicked() {
                        self.execute_unsaved_action(action.clone());
                        resolved = true;
                    }
                    if !resolved && ui.button("Cancel").clicked() {
                        resolved = true;
                    }
                });
            });
        if !open || resolved {
            self.ui.dialogs.pending_unsaved_action = None;
        }
    }

    /// If the document has unsaved changes, store the action for later
    /// resolution; otherwise execute it immediately.
    pub(crate) fn guard_unsaved(&mut self, action: UnsavedWarningAction) {
        if self.document.dirty_since_last_autosave {
            self.ui.dialogs.pending_unsaved_action = Some(action);
        } else {
            self.execute_unsaved_action(action);
        }
    }

    /// Execute a deferred destructive action after the user has resolved the
    /// unsaved-changes warning (or after a save completes).
    pub(crate) fn execute_unsaved_action(&mut self, action: UnsavedWarningAction) {
        match action {
            UnsavedWarningAction::Quit => {
                self.ui.should_close = true;
            }
            UnsavedWarningAction::NewCanvas => {
                self.ui.dialogs.show_new_canvas_dialog = true;
            }
            UnsavedWarningAction::Load => {
                self.dialog_manager.queue_file_action(PendingFileAction::Load);
            }
            UnsavedWarningAction::Import => {
                self.dialog_manager.queue_file_action(PendingFileAction::Import);
            }
            UnsavedWarningAction::LoadPath(path) => {
                self.dialog_manager.queue_load_direct(path);
            }
        }
    }

    /// Show the stamp-naming dialog when a new stamp has been loaded.
    pub(crate) fn show_stamp_naming_dialog(&mut self, ui: &mut egui::Ui) {
        let Some(mut pending) = self.ui.dialogs.pending_stamp_name.take() else {
            return;
        };
        let mut open = true;
        let mut cancelled = false;
        let label = format!("Size: {}×{}", pending.width, pending.height);
        egui::Window::new("Name Your Stamp")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut pending.name).id_source("stamp_name_text"),
                    );
                });
                ui.label(&label);
                if ui.button("Cancel").clicked() {
                    cancelled = true;
                }
                if ui.button("Add Stamp").clicked() && !pending.name.is_empty() {
                    let stamp_name = pending.name.clone();
                    let stamp_pixels = std::mem::take(&mut pending.pixels);
                    let stamp_w = pending.width;
                    let stamp_h = pending.height;
                    crate::stamp_library::add_stamp(
                        &mut self.stamp_library,
                        stamp_name.clone(),
                        stamp_pixels,
                        stamp_w,
                        stamp_h,
                        ui.ctx(),
                    );
                    self.ui.toasts.message =
                        Some((format!("Stamp \"{stamp_name}\" added"), Instant::now()));
                }
            });
        if open && !cancelled {
            self.ui.dialogs.pending_stamp_name = Some(pending);
        }
    }

    /// Show the brush-import naming dialog when brushes have been loaded.
    pub(crate) fn show_brush_naming_dialog(&mut self, ui: &mut egui::Ui) {
        let Some(brushes) = &mut self.ui.dialogs.pending_brushes else {
            return;
        };
        let mut open = true;
        let mut confirmed = false;
        let mut cancelled = false;
        egui::Window::new("Name Your Brushes")
            .collapsible(false)
            .resizable(true)
            .default_size([400.0, 300.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.label(format!(
                    "{} brush(es) imported — edit names below:",
                    brushes.len()
                ));
                ui.separator();

                let mut names_to_remove: Vec<usize> = Vec::new();
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height() - 40.0)
                    .show(ui, |ui| {
                        for (i, brush) in brushes.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut brush.name)
                                        .desired_width(150.0)
                                        .id_source(format!("brush_name_{i}")),
                                );
                                ui.label(format!("{}×{}", brush.width, brush.height));
                                if ui.button("Remove").clicked() {
                                    names_to_remove.push(i);
                                }
                            });
                        }
                    });

                names_to_remove.sort_unstable_by(|a, b| b.cmp(a));
                for i in names_to_remove {
                    brushes.remove(i);
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        cancelled = true;
                    }
                    if ui.button("Import All").clicked() && !brushes.is_empty() {
                        confirmed = true;
                    }
                });
            });
        if cancelled {
            self.ui.dialogs.pending_brushes = None;
        } else if confirmed {
            let Some(all_brushes) = self.ui.dialogs.pending_brushes.take() else {
                return;
            };
            let count = all_brushes.len();
            for brush in all_brushes {
                if !brush.name.is_empty() {
                    crate::brush_library::add_brush(
                        &mut self.brush_library,
                        brush.name,
                        brush.pixels,
                        brush.width,
                        brush.height,
                        brush.spacing,
                        ui.ctx(),
                    );
                }
            }
            self.ui.toasts.message = Some((format!("Imported {count} brush(es)"), Instant::now()));
        } else if !open {
            self.ui.dialogs.pending_brushes = None;
        }
    }

    /// Show a brief toast notification (auto-dismissed after 2 seconds).
    pub(crate) fn show_toast(&mut self, ui: &mut egui::Ui) {
        let Some((message, triggered_at)) = &self.ui.toasts.message.clone() else {
            return;
        };
        if triggered_at.elapsed() < std::time::Duration::from_secs(2) {
            egui::Area::new(egui::Id::new("stamp_toast"))
                .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(message)
                            .color(egui::Color32::WHITE)
                            .background_color(egui::Color32::from_black_alpha(180)),
                    );
                });
        } else {
            self.ui.toasts.message = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::PendingStamp;
    use crate::app::UnsavedWarningAction;
    use egui_kittest::kittest::Queryable;

    /// Helper: create an app with a temp data dir.
    fn test_app() -> (crate::app::MyApp, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("temp dir");
        let app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        (app, dir)
    }

    // --- show_error_window ---

    #[test]
    fn show_error_window_renders_errors() {
        let (mut app, _dir) = test_app();
        app.ui.errors.list.push("test error".into());

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_error_window(ui);
        });
        harness.step();

        let _error = harness.get_by_label("Error: test error");
        let _dismiss = harness.get_by_label("Dismiss");
        let _copy = harness.get_by_label("Copy error");
        let _all = harness.get_by_label("Dismiss All");
    }

    #[test]
    fn show_error_window_dismiss_clears_error() {
        let (mut app, _dir) = test_app();
        app.ui.errors.list.push("single error".into());

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_error_window(ui);
        });
        harness.step();

        harness.get_by_label("Dismiss").click();
        harness.step();

        drop(harness);
        assert!(app.ui.errors.list.is_empty());
    }

    // --- show_delete_layer_dialog ---

    #[test]
    fn show_delete_layer_dialog_renders() {
        let (mut app, _dir) = test_app();
        app.ui.dialogs.show_delete_layer_dialog = Some(0);

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_delete_layer_dialog(ui);
        });
        harness.step();

        let _delete = harness.get_by_label("Delete");
        let _cancel = harness.get_by_label("Cancel");
    }

    #[test]
    fn show_delete_layer_dialog_cancel_clears_flag() {
        let (mut app, _dir) = test_app();
        app.ui.dialogs.show_delete_layer_dialog = Some(0);

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_delete_layer_dialog(ui);
        });
        harness.step();

        harness.get_by_label("Cancel").click();
        harness.step();

        drop(harness);
        assert!(app.ui.dialogs.show_delete_layer_dialog.is_none());
    }

    // --- show_new_canvas_dialog ---

    #[test]
    fn show_new_canvas_dialog_renders_presets() {
        let (mut app, _dir) = test_app();
        app.ui.dialogs.show_new_canvas_dialog = true;

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_new_canvas_dialog(ui);
        });
        harness.step();

        let _create = harness.get_by_label("Create");
        let _cancel = harness.get_by_label("Cancel");
    }

    #[test]
    fn show_new_canvas_dialog_cancel_closes() {
        let (mut app, _dir) = test_app();
        app.ui.dialogs.show_new_canvas_dialog = true;

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_new_canvas_dialog(ui);
        });
        harness.step();

        harness.get_by_label("Cancel").click();
        harness.step();

        drop(harness);
        assert!(!app.ui.dialogs.show_new_canvas_dialog);
    }

    // --- show_unsaved_changes_warning ---

    #[test]
    fn show_unsaved_changes_warning_renders() {
        let (mut app, _dir) = test_app();
        app.ui.dialogs.pending_unsaved_action = Some(UnsavedWarningAction::Quit);

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_unsaved_changes_warning(ui);
        });
        harness.step();

        harness.get_by_label("Unsaved Changes");
        harness.get_by_label("Cancel");
    }

    // --- guard_unsaved / execute_unsaved_action ---

    #[test]
    fn guard_unsaved_dirty_stores_action() {
        let (mut app, _dir) = test_app();
        app.document.dirty_since_last_autosave = true;

        app.guard_unsaved(UnsavedWarningAction::Quit);

        assert!(matches!(
            app.ui.dialogs.pending_unsaved_action,
            Some(UnsavedWarningAction::Quit)
        ));
    }

    #[test]
    fn guard_unsaved_clean_executes_directly() {
        let (mut app, _dir) = test_app();
        app.document.dirty_since_last_autosave = false;
        app.ui.should_close = false;

        app.guard_unsaved(UnsavedWarningAction::Quit);

        assert!(app.ui.dialogs.pending_unsaved_action.is_none());
        assert!(app.ui.should_close);
    }

    #[test]
    fn execute_unsaved_action_quit_sets_should_close() {
        let (mut app, _dir) = test_app();
        app.execute_unsaved_action(UnsavedWarningAction::Quit);
        assert!(app.ui.should_close);
    }

    #[test]
    fn execute_unsaved_action_new_canvas_opens_dialog() {
        let (mut app, _dir) = test_app();
        app.execute_unsaved_action(UnsavedWarningAction::NewCanvas);
        assert!(app.ui.dialogs.show_new_canvas_dialog);
    }

    // --- show_large_canvas_warning ---

    #[test]
    fn show_large_canvas_warning_renders() {
        let (mut app, _dir) = test_app();
        app.ui.dialogs.pending_large_canvas = Some((100, 100));

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_large_canvas_warning(ui);
        });
        harness.step();

        let _yes = harness.get_by_label("Yes, create");
        let _cancel = harness.get_by_label("Cancel");
    }

    #[test]
    fn show_large_canvas_warning_cancel_clears_flag() {
        let (mut app, _dir) = test_app();
        app.ui.dialogs.pending_large_canvas = Some((200, 200));

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_large_canvas_warning(ui);
        });
        harness.step();

        harness.get_by_label("Cancel").click();
        harness.step();

        drop(harness);
        assert!(app.ui.dialogs.pending_large_canvas.is_none());
    }

    // --- show_stamp_naming_dialog ---

    #[test]
    fn show_stamp_naming_dialog_renders() {
        let (mut app, _dir) = test_app();
        app.ui.dialogs.pending_stamp_name = Some(PendingStamp {
            name: "mystamp".into(),
            pixels: vec![],
            width: 32,
            height: 32,
            spacing: 0,
        });

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_stamp_naming_dialog(ui);
        });
        harness.step();

        harness.get_by_label("Name Your Stamp");
        harness.get_by_label("Size: 32×32");
        harness.get_by_label("Cancel");
        harness.get_by_label("Add Stamp");
    }

    #[test]
    fn show_stamp_naming_dialog_cancel_clears_pending() {
        let (mut app, _dir) = test_app();
        app.ui.dialogs.pending_stamp_name = Some(PendingStamp {
            name: "test".into(),
            pixels: vec![],
            width: 16,
            height: 16,
            spacing: 0,
        });

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_stamp_naming_dialog(ui);
        });
        harness.step();

        harness.get_by_label("Cancel").click();
        harness.step();

        drop(harness);
        assert!(app.ui.dialogs.pending_stamp_name.is_none());
    }

    // --- show_brush_naming_dialog ---

    #[test]
    fn show_brush_naming_dialog_renders() {
        let (mut app, _dir) = test_app();
        app.ui.dialogs.pending_brushes = Some(vec![PendingStamp {
            name: "brush1".into(),
            pixels: vec![],
            width: 8,
            height: 8,
            spacing: 50,
        }]);

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_brush_naming_dialog(ui);
        });
        harness.step();

        harness.get_by_label("Name Your Brushes");
        harness.get_by_label("Cancel");
        harness.get_by_label("Import All");
    }

    #[test]
    fn show_brush_naming_dialog_cancel_clears_pending() {
        let (mut app, _dir) = test_app();
        app.ui.dialogs.pending_brushes = Some(vec![PendingStamp {
            name: "b".into(),
            pixels: vec![],
            width: 4,
            height: 4,
            spacing: 25,
        }]);

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_brush_naming_dialog(ui);
        });
        harness.step();

        harness.get_by_label("Cancel").click();
        harness.step();

        drop(harness);
        assert!(app.ui.dialogs.pending_brushes.is_none());
    }

    // --- show_toast ---

    use std::time::Instant;

    #[test]
    fn show_toast_renders_message() {
        let (mut app, _dir) = test_app();
        app.ui.toasts.message = Some(("Hello world".into(), Instant::now()));

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_toast(ui);
        });
        harness.step();

        let _msg = harness.get_by_label("Hello world");
    }

    #[test]
    fn show_toast_expired_clears_message() {
        let (mut app, _dir) = test_app();
        // 99 seconds ago — well past the 2-second expiry.
        let past = Instant::now()
            .checked_sub(std::time::Duration::from_secs(99))
            .unwrap();
        app.ui.toasts.message = Some(("Expired".into(), past));

        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            app.show_toast(ui);
        });
        harness.step();

        drop(harness);
        assert!(app.ui.toasts.message.is_none());
    }
}
