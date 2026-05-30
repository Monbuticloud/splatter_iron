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

    /// Show the stamp-naming dialog when a new stamp has been loaded.
    pub(crate) fn show_stamp_naming_dialog(&mut self, ui: &mut egui::Ui) {
        let Some(mut pending) = self.ui.dialogs.pending_stamp_name.take() else {
            return;
        };
        let mut open = true;
        let mut cancelled = false;
        let label = format!("Size: {}×{}", pending.width, pending.height);
        egui::Window
            ::new("Name Your Stamp")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut pending.name).id_source("stamp_name_text")
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
                        ui.ctx()
                    );
                    self.ui.toasts.message = Some((
                        format!("Stamp \"{stamp_name}\" added"),
                        Instant::now(),
                    ));
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
        egui::Window
            ::new("Name Your Brushes")
            .collapsible(false)
            .resizable(true)
            .default_size([400.0, 300.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                ui.label(format!("{} brush(es) imported — edit names below:", brushes.len()));
                ui.separator();

                let mut names_to_remove: Vec<usize> = Vec::new();
                egui::ScrollArea
                    ::vertical()
                    .max_height(ui.available_height() - 40.0)
                    .show(ui, |ui| {
                        for (i, brush) in brushes.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit
                                        ::singleline(&mut brush.name)
                                        .desired_width(150.0)
                                        .id_source(format!("brush_name_{i}"))
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
                        ui.ctx()
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
            egui::Area
                ::new(egui::Id::new("stamp_toast"))
                .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText
                            ::new(message)
                            .color(egui::Color32::WHITE)
                            .background_color(egui::Color32::from_black_alpha(180))
                    );
                });
        } else {
            self.ui.toasts.message = None;
        }
    }

}
