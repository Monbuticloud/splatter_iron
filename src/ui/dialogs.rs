//! Dialog windows: error list, large-canvas warning, delete-layer
//! confirmation, new-canvas presets, unsaved-changes warning, stamp naming,
//! brush naming, toast notifications, and progress indicator.

use std::time::Instant;

use eframe::egui;

use crate::app::estimate_canvas_memory;
use crate::app::MEMORY_WARNING_THRESHOLD;
use crate::app::MyApp;
use crate::app::NEW_CANVAS_PRESETS;
use crate::app::PendingStamp;
use crate::app::UnsavedWarningAction;
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
        egui::Window
            ::new("Error")
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
        let estimated = estimate_canvas_memory(w, h);
        let estimated_mb = estimated / (1024 * 1024);
        egui::Window
            ::new("Large Canvas Warning")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.label(
                    format!(
                        "This canvas ({w}×{h}) may use up to ~{estimated_mb} MB of RAM.\n\
                     Proceed? This cannot be undone."
                    )
                );
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
        let layer_name = self.document.canvas.pixels
            .get(index)
            .map(|l| {
                if l.name.is_empty() { format!("Layer {index}") } else { l.name.clone() }
            })
            .unwrap_or_default();
        let mut open = true;
        egui::Window
            ::new("Delete Layer")
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
        egui::Window
            ::new("New Canvas")
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
                    egui::Slider
                        ::new(
                            &mut self.ui.dialogs.new_canvas_width,
                            4..=self.ui.max_texture_dimension
                        )
                        .text("Width")
                );
                ui.add(
                    egui::Slider
                        ::new(
                            &mut self.ui.dialogs.new_canvas_height,
                            4..=self.ui.max_texture_dimension
                        )
                        .text("Height")
                );
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        if self.document.dirty_since_last_autosave {
                            self.ui.dialogs.pending_unsaved_action = Some(
                                UnsavedWarningAction::NewCanvas
                            );
                        } else {
                            let mem = estimate_canvas_memory(
                                self.ui.dialogs.new_canvas_width,
                                self.ui.dialogs.new_canvas_height
                            );
                            if mem > MEMORY_WARNING_THRESHOLD {
                                self.ui.dialogs.pending_large_canvas = Some((
                                    self.ui.dialogs.new_canvas_width,
                                    self.ui.dialogs.new_canvas_height,
                                ));
                            } else {
                                let canvas = Canvas::new(
                                    self.ui.dialogs.new_canvas_width,
                                    self.ui.dialogs.new_canvas_height
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
        if self.ui.dialogs.pending_unsaved_action.is_none() {
            return;
        }
        let action = self.ui.dialogs.pending_unsaved_action.as_ref().unwrap().clone();
        let mut open = true;
        let mut resolved = false;
        let label: String = if self.document.savefile_path.is_empty() {
            "You have unsaved changes. What would you like to do?".to_string()
        } else {
            format!(
                "\"{}\" has unsaved changes. What would you like to do?",
                std::path::Path
                    ::new(&self.document.savefile_path)
                    .file_name()
                    .map(|s| s.to_string_lossy())
                    .unwrap_or_default()
            )
        };
        egui::Window
            ::new("Unsaved Changes")
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
                            self.file_io.queue_file_action(PendingFileAction::Save);
                        } else {
                            self.file_io.save_to_current_path(&mut self.document);
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
                self.file_io.queue_file_action(PendingFileAction::Load);
            }
            UnsavedWarningAction::Import => {
                self.file_io.queue_file_action(PendingFileAction::Import);
            }
            UnsavedWarningAction::LoadPath(path) => {
                self.file_io.queue_load_direct(path);
            }
        }
    }
}
